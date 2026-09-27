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
        /** Chunks written at least once; later changes come from block updates */
        final Set<Long> captured = ConcurrentHashMap.newKeySet();
        int ticks;
    }

    private static final Map<ServerLevel, State> STATES = Collections.synchronizedMap(new WeakHashMap<>());
    private static final ExecutorService WRITER = Executors.newSingleThreadExecutor(runnable -> {
        Thread thread = new Thread(runnable, "Ingot live map");
        thread.setDaemon(true);
        return thread;
    });
    private static final Path LIVE_DIR = LiveChunk.liveDir();

    private LiveTracker() {}

    private static long key(int chunkX, int chunkZ) {
        return (chunkX & 0xFFFFFFFFL) | ((chunkZ & 0xFFFFFFFFL) << 32);
    }

    private static State state(ServerLevel level) {
        return STATES.computeIfAbsent(level, l -> new State());
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
                    if (!state.captured.contains(key) && level.getChunkSource().getChunkNow(x, z) != null) {
                        state.dirty.add(key);
                    }
                }
            }
        }
    }

    private static void drain(ServerLevel level, State state) {
        Iterator<Long> it = state.dirty.iterator();
        for (int i = 0; i < CHUNKS_PER_DRAIN && it.hasNext(); i++) {
            long key = it.next();
            it.remove();
            int x = (int) key;
            int z = (int) (key >>> 32);
            LevelChunk chunk = level.getChunkSource().getChunkNow(x, z);
            if (chunk == null) continue;
            state.captured.add(key);
            capture(level, chunk, x, z);
        }
    }

    private static void capture(ServerLevel level, LevelChunk chunk, int chunkX, int chunkZ) {
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
                    (BlockState state) -> BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString(),
                    BlockState::isAir,
                    BlockState::canOcclude,
                    level.dimensionType().hasCeiling(),
                    level.getMinY(),
                    level.getMaxY());
        } catch (Exception e) {
            return;
        }
        WRITER.execute(() -> {
            try {
                LiveChunk.write(LIVE_DIR, dimension, chunkX, chunkZ, data);
            } catch (Exception ignored) {
                // A missed write only means the map shows the saved chunk until the next change
            }
        });
    }
}
