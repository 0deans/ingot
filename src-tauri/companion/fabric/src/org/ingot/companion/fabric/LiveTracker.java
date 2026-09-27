package org.ingot.companion.fabric;

import java.util.Collections;
import java.util.Iterator;
import java.util.Map;
import java.util.Set;
import java.util.WeakHashMap;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.nio.file.Path;

import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.chunk.LevelChunk;
import net.minecraft.world.level.levelgen.Heightmap;
import org.ingot.companion.LiveChunk;

/**
 * Feeds Ingot's live map on Fabric: chunks near players are captured once they're loaded,
 * and again whenever a block in them changes. Reads happen on the server thread during
 * the world's tick; encoding results are written off-thread. Never saves the world.
 */
public final class LiveTracker {
    /** Ticks between drains, and chunks captured per drain (spreads bursts over ticks) */
    private static final int DRAIN_EVERY = 5;
    private static final int CHUNKS_PER_DRAIN = 16;
    /** Ticks between looking for newly loaded chunks around players */
    private static final int SCAN_EVERY = 40;
    private static final int MAX_SCAN_RADIUS = 12;

    private static final class State {
        /** Chunks to capture (packed x/z) */
        final Set<Long> dirty = ConcurrentHashMap.newKeySet();
        /** What was written and when: skips identical and too-frequent writes */
        final LiveChunk.Tracker tracker = new LiveChunk.Tracker();
        int ticks;
    }

    private static final Map<ServerLevel, State> STATES = Collections.synchronizedMap(new WeakHashMap<>());
    private static final ExecutorService WRITER = Executors.newSingleThreadExecutor(runnable -> {
        Thread thread = new Thread(runnable, "Ingot live map");
        thread.setDaemon(true);
        return thread;
    });
    private static final Path LIVE_DIR = LiveChunk.liveDir();

    /** The last level looked up: block changes come in bursts from the same world */
    private static volatile ServerLevel lastLevel;
    private static volatile State lastState;

    private LiveTracker() {}

    private static long key(int chunkX, int chunkZ) {
        return LiveChunk.key(chunkX, chunkZ);
    }

    private static State state(ServerLevel level) {
        State cached = lastState;
        if (lastLevel == level && cached != null) return cached;
        State state = STATES.computeIfAbsent(level, l -> new State());
        lastState = state;
        lastLevel = level;
        return state;
    }

    /** From LevelChunk.setBlockState: every block change goes through it */
    public static void blockChanged(Level level, BlockPos pos) {
        if (level instanceof ServerLevel server) {
            state(server).dirty.add(key(pos.getX() >> 4, pos.getZ() >> 4));
        }
    }

    /** From ServerLevel.tick */
    public static void tick(ServerLevel level) {
        State state = state(level);
        state.ticks++;
        if (state.ticks % SCAN_EVERY == 0) scanAroundPlayers(level, state);
        if (state.ticks % DRAIN_EVERY == 0) drain(level, state);
    }

    private static void scanAroundPlayers(ServerLevel level, State state) {
        int radius = Math.min(level.getServer().getPlayerList().getViewDistance(), MAX_SCAN_RADIUS);
        for (ServerPlayer player : level.players()) {
            ChunkPos center = player.chunkPosition();
            for (int dx = -radius; dx <= radius; dx++) {
                for (int dz = -radius; dz <= radius; dz++) {
                    int x = center.x() + dx;
                    int z = center.z() + dz;
                    long key = key(x, z);
                    if (!state.tracker.captured(key) && level.getChunkSource().getChunkNow(x, z) != null) {
                        state.dirty.add(key);
                    }
                }
            }
        }
    }

    private static void drain(ServerLevel level, State state) {
        long now = System.currentTimeMillis();
        Iterator<Long> it = state.dirty.iterator();
        int captured = 0;
        // Bounded, so a queue full of cooling-down chunks can't turn into a long scan
        for (int scanned = 0; captured < CHUNKS_PER_DRAIN && scanned < CHUNKS_PER_DRAIN * 4 && it.hasNext(); scanned++) {
            long key = it.next();
            // Written moments ago: stays queued and is captured once the cooldown passes
            if (state.tracker.coolingDown(key, now)) continue;
            it.remove();
            int x = (int) key;
            int z = (int) (key >>> 32);
            LevelChunk chunk = level.getChunkSource().getChunkNow(x, z);
            if (chunk == null) continue;
            captured++;
            capture(level, state, chunk, x, z, now);
        }
    }

    private static void capture(ServerLevel level, State state, LevelChunk chunk, int chunkX, int chunkZ, long now) {
        BlockPos.MutableBlockPos pos = new BlockPos.MutableBlockPos();
        int baseX = chunkX << 4;
        int baseZ = chunkZ << 4;
        String dimension = level.dimension().identifier().toString();
        byte[] data;
        try {
            data = LiveChunk.encode(
                    (x, y, z) -> chunk.getBlockState(pos.set(baseX + x, y, baseZ + z)),
                    // The y of the highest non-air block (ChunkAccess already subtracts
                    // one from the heightmap's first free y)
                    (x, z) -> chunk.getHeight(Heightmap.Types.WORLD_SURFACE, x, z),
                    (BlockState block) -> BuiltInRegistries.BLOCK.getKey(block.getBlock()).toString(),
                    BlockState::isAir,
                    BlockState::canOcclude,
                    level.dimensionType().hasCeiling(),
                    level.getMinY(),
                    level.getMaxY());
        } catch (Exception e) {
            return;
        }
        // Most changes are below the surface: nothing new to write
        if (!state.tracker.changed(key(chunkX, chunkZ), data, now)) return;
        WRITER.execute(() -> {
            try {
                LiveChunk.write(LIVE_DIR, dimension, chunkX, chunkZ, data);
            } catch (Exception ignored) {
                // A missed write only means the map shows the saved chunk until the next change
            }
        });
    }
}
