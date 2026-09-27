package org.ingot.companion;

import java.io.IOException;
import java.nio.file.Path;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

import org.bukkit.Chunk;
import org.bukkit.ChunkSnapshot;
import org.bukkit.Location;
import org.bukkit.Material;
import org.bukkit.World;
import org.bukkit.entity.Player;
import org.bukkit.block.Block;
import org.bukkit.block.BlockState;
import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.block.BlockBreakEvent;
import org.bukkit.event.block.BlockBurnEvent;
import org.bukkit.event.block.BlockExplodeEvent;
import org.bukkit.event.block.BlockFadeEvent;
import org.bukkit.event.block.BlockFormEvent;
import org.bukkit.event.block.BlockFromToEvent;
import org.bukkit.event.block.BlockGrowEvent;
import org.bukkit.event.block.BlockPistonExtendEvent;
import org.bukkit.event.block.BlockPistonRetractEvent;
import org.bukkit.event.block.BlockPlaceEvent;
import org.bukkit.event.block.BlockSpreadEvent;
import org.bukkit.event.block.LeavesDecayEvent;
import org.bukkit.event.entity.EntityChangeBlockEvent;
import org.bukkit.event.entity.EntityExplodeEvent;
import org.bukkit.event.player.PlayerBucketEmptyEvent;
import org.bukkit.event.player.PlayerBucketFillEvent;
import org.bukkit.event.world.ChunkLoadEvent;
import org.bukkit.event.world.ChunkUnloadEvent;
import org.bukkit.event.world.StructureGrowEvent;
import org.bukkit.plugin.java.JavaPlugin;

/**
 * Feeds Ingot's live map. Reads the surface of chunks straight from memory whenever
 * they load, change or unload, and writes them in Ingot's live chunk format (see
 * {@link LiveChunk}). Never saves the world; Ingot shows whichever is newer, this or the
 * saved chunk.
 *
 * <p>Uses Paper's region schedulers, so the same jar runs on Paper, Purpur and Folia.
 */
public final class IngotCompanion extends JavaPlugin implements Listener {
    /** Chunks snapshotted per drain; spreads bursts (flying, explosions) over ticks */
    private static final int CHUNKS_PER_DRAIN = 48;
    /**
     * Ticks between re-checking the chunks around each player, and how far. Events don't
     * cover commands (/fill, /setblock), WorldEdit or other plugins changing blocks; this
     * catches those. Unchanged chunks cost a snapshot but no write (see LiveChunk.Tracker).
     */
    private static final long SWEEP_TICKS = 200L;
    private static final int SWEEP_RADIUS = 3;

    private record ChunkKey(UUID world, int x, int z) {}

    private final Set<ChunkKey> dirty = ConcurrentHashMap.newKeySet();
    /** Per world: what was written, and when (skips identical and too-frequent writes) */
    private final Map<UUID, LiveChunk.Tracker> trackers = new ConcurrentHashMap<>();
    private Path liveDir;

    private LiveChunk.Tracker tracker(World world) {
        return trackers.computeIfAbsent(world.getUID(), id -> new LiveChunk.Tracker());
    }

    @Override
    public void onEnable() {
        liveDir = LiveChunk.liveDir();
        getServer().getPluginManager().registerEvents(this, this);
        getServer().getGlobalRegionScheduler().runAtFixedRate(this, task -> drain(), 20L, 10L);
        getServer().getGlobalRegionScheduler().runAtFixedRate(this, task -> sweep(), SWEEP_TICKS, SWEEP_TICKS);
        // Everything loaded before the plugin (spawn area, players already online)
        for (World world : getServer().getWorlds()) {
            try {
                for (Chunk chunk : world.getLoadedChunks()) markDirty(world, chunk.getX(), chunk.getZ());
            } catch (UnsupportedOperationException ignored) {
                // Folia can't list loaded chunks from here; they're captured as they change
            }
        }
    }

    // ─── Change tracking ─────────────────────────────────────────────────────

    private void markDirty(World world, int chunkX, int chunkZ) {
        dirty.add(new ChunkKey(world.getUID(), chunkX, chunkZ));
    }

    private void markDirty(Block block) {
        markDirty(block.getWorld(), block.getX() >> 4, block.getZ() >> 4);
    }

    private void markDirty(Location location) {
        if (location.getWorld() != null) {
            markDirty(location.getWorld(), location.getBlockX() >> 4, location.getBlockZ() >> 4);
        }
    }

    private void markDirty(List<Block> blocks) {
        for (Block block : blocks) markDirty(block);
    }

    @EventHandler(priority = EventPriority.MONITOR)
    public void onChunkLoad(ChunkLoadEvent event) {
        int x = event.getChunk().getX();
        int z = event.getChunk().getZ();
        // Already captured this run: it can't have changed while it was unloaded
        if (tracker(event.getWorld()).captured(LiveChunk.key(x, z))) return;
        markDirty(event.getWorld(), x, z);
    }

    /** Last chance to capture the chunk; runs on the thread that owns it */
    @EventHandler(priority = EventPriority.MONITOR)
    public void onChunkUnload(ChunkUnloadEvent event) {
        Chunk chunk = event.getChunk();
        ChunkKey key = new ChunkKey(event.getWorld().getUID(), chunk.getX(), chunk.getZ());
        if (dirty.remove(key)) capture(event.getWorld(), chunk);
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onPlace(BlockPlaceEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onBreak(BlockBreakEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onBurn(BlockBurnEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onBlockExplode(BlockExplodeEvent event) { markDirty(event.blockList()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onEntityExplode(EntityExplodeEvent event) { markDirty(event.blockList()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onFade(BlockFadeEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onForm(BlockFormEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onFlow(BlockFromToEvent event) { markDirty(event.getToBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onGrow(BlockGrowEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onSpread(BlockSpreadEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onDecay(LeavesDecayEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onPistonExtend(BlockPistonExtendEvent event) {
        markDirty(event.getBlock());
        markDirty(event.getBlocks());
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onPistonRetract(BlockPistonRetractEvent event) {
        markDirty(event.getBlock());
        markDirty(event.getBlocks());
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onEntityChange(EntityChangeBlockEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onBucketEmpty(PlayerBucketEmptyEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onBucketFill(PlayerBucketFillEvent event) { markDirty(event.getBlock()); }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    public void onStructureGrow(StructureGrowEvent event) {
        markDirty(event.getLocation());
        for (BlockState state : event.getBlocks()) markDirty(state.getLocation());
    }

    /** Queues the chunks around each player, reading their position on their own thread */
    private void sweep() {
        for (Player player : getServer().getOnlinePlayers()) {
            player.getScheduler().run(this, task -> {
                World world = player.getWorld();
                int cx = player.getLocation().getBlockX() >> 4;
                int cz = player.getLocation().getBlockZ() >> 4;
                for (int dx = -SWEEP_RADIUS; dx <= SWEEP_RADIUS; dx++) {
                    for (int dz = -SWEEP_RADIUS; dz <= SWEEP_RADIUS; dz++) markDirty(world, cx + dx, cz + dz);
                }
            }, null);
        }
    }

    // ─── Capturing ───────────────────────────────────────────────────────────

    /** Snapshots a batch of changed chunks, each on the thread that owns it */
    private void drain() {
        long now = System.currentTimeMillis();
        Iterator<ChunkKey> it = dirty.iterator();
        int scheduled = 0;
        // Bounded, so a queue full of cooling-down chunks can't turn into a long scan
        for (int scanned = 0; scheduled < CHUNKS_PER_DRAIN && scanned < CHUNKS_PER_DRAIN * 4 && it.hasNext(); scanned++) {
            ChunkKey key = it.next();
            World world = getServer().getWorld(key.world());
            if (world == null) {
                it.remove();
                continue;
            }
            // Written moments ago: stays queued and is captured once the cooldown passes
            if (tracker(world).coolingDown(LiveChunk.key(key.x(), key.z()), now)) continue;
            it.remove();
            scheduled++;
            getServer().getRegionScheduler().run(this, world, key.x(), key.z(), task -> {
                if (world.isChunkLoaded(key.x(), key.z())) capture(world, world.getChunkAt(key.x(), key.z()));
            });
        }
    }

    /** Takes a snapshot here (owning thread) and encodes/writes it off-thread */
    private void capture(World world, Chunk chunk) {
        ChunkSnapshot snapshot = chunk.getChunkSnapshot(true, false, false);
        boolean nether = world.getEnvironment() == World.Environment.NETHER;
        int minY = world.getMinHeight();
        int maxY = world.getMaxHeight() - 1;
        String dimension = world.getKey().toString();
        LiveChunk.Tracker tracker = tracker(world);
        long key = LiveChunk.key(chunk.getX(), chunk.getZ());
        getServer().getAsyncScheduler().runNow(this, task -> {
            try {
                byte[] data = LiveChunk.encode(
                        snapshot::getBlockType,
                        snapshot::getHighestBlockYAt,
                        material -> material.getKey().toString(),
                        Material::isAir,
                        Material::isOccluding,
                        nether,
                        minY,
                        maxY);
                // Most changes are below the surface: nothing new to write
                if (!tracker.changed(key, data, System.currentTimeMillis())) return;
                LiveChunk.write(liveDir, dimension, chunk.getX(), chunk.getZ(), data);
            } catch (IOException | RuntimeException e) {
                getLogger().fine("Couldn't write live map data: " + e);
            }
        });
    }

    @Override
    public void onDisable() {
        getServer().getAsyncScheduler().cancelTasks(this);
        getServer().getGlobalRegionScheduler().cancelTasks(this);
        dirty.clear();
    }
}
