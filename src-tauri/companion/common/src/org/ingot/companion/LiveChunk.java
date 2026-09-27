package org.ingot.companion;

import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Function;
import java.util.function.Predicate;

/**
 * Ingot's live chunk format, shared by the Paper plugin and the Fabric mod so both write
 * exactly what Ingot's map reads: one file per chunk at
 * {@code .ingot/live/<namespace>/<dimension>/<rx>.<rz>/<cx>.<cz>.bin} in the server folder.
 *
 * <p>Per column it stores the blocks from the surface down to the first opaque one, as runs
 * of equal blocks; Ingot colors them with its own rules, so live and saved chunks match.
 * Format (big endian): "IGC" + version byte, palette (u16 count, UTF strings), then 256
 * columns (index z * 16 + x): u16 run count, runs of (u16 palette index, i16 top y,
 * u16 length) from the top down.
 */
public final class LiveChunk {
    /** Blocks walked down per column at most (deep oceans are one long water run) */
    private static final int MAX_DEPTH = 384;

    private LiveChunk() {}

    /** A block in the chunk at local x/z (0-15) and world y */
    public interface Blocks<T> {
        T at(int x, int y, int z);
    }

    /** Local x/z (0-15) to the highest non-air block's y */
    public interface Surface {
        int topY(int x, int z);
    }

    /**
     * @param id namespaced block id, e.g. "minecraft:grass_block"
     * @param nether read below the bedrock roof, like Ingot's renderer does
     */
    public static <T> byte[] encode(
            Blocks<T> blocks,
            Surface surface,
            Function<T, String> id,
            Predicate<T> air,
            Predicate<T> opaque,
            boolean nether,
            int minY,
            int maxY) throws IOException {
        Map<String, Integer> palette = new HashMap<>();
        List<String> names = new ArrayList<>();
        List<int[]> columns = new ArrayList<>(256);
        for (int z = 0; z < 16; z++) {
            for (int x = 0; x < 16; x++) {
                int top = nether ? Math.min(127, maxY) : Math.min(surface.topY(x, z), maxY);
                List<Integer> runs = new ArrayList<>();
                boolean belowRoof = !nether;
                int lastIndex = -1;
                int walked = 0;
                for (int y = top; y >= minY && walked < MAX_DEPTH; y--, walked++) {
                    T block = blocks.at(x, y, z);
                    if (!belowRoof) {
                        if (air.test(block)) belowRoof = true;
                        continue;
                    }
                    int index = palette.computeIfAbsent(id.apply(block), name -> {
                        names.add(name);
                        return names.size() - 1;
                    });
                    if (index == lastIndex) {
                        runs.set(runs.size() - 1, runs.get(runs.size() - 1) + 1);
                    } else {
                        runs.add(index);
                        runs.add(y);
                        runs.add(1);
                        lastIndex = index;
                    }
                    if (opaque.test(block)) break;
                }
                columns.add(runs.stream().mapToInt(Integer::intValue).toArray());
            }
        }

        ByteArrayOutputStream bytes = new ByteArrayOutputStream(4096);
        try (DataOutputStream out = new DataOutputStream(bytes)) {
            out.writeBytes("IGC");
            out.writeByte(1);
            out.writeShort(names.size());
            for (String name : names) out.writeUTF(name);
            for (int[] runs : columns) {
                out.writeShort(runs.length / 3);
                for (int value : runs) out.writeShort(value);
            }
        }
        return bytes.toByteArray();
    }

    /** Chunk x/z packed into one key */
    public static long key(int chunkX, int chunkZ) {
        return (chunkX & 0xFFFFFFFFL) | ((chunkZ & 0xFFFFFFFFL) << 32);
    }

    /**
     * Per-dimension bookkeeping that keeps the server's work down. Ingot re-renders a map
     * region whenever one of its chunk files changes, so writes are worth avoiding:
     * <ul>
     *   <li>most block changes (mining, redstone, chests) don't change the surface, and
     *       an identical file isn't written again</li>
     *   <li>a chunk that keeps changing (flowing water, farms) is written at most every
     *       {@link #MIN_INTERVAL_MS}; the map only looks every few seconds anyway</li>
     * </ul>
     * Thread-safe: captures and writes may happen on different threads.
     */
    public static final class Tracker {
        public static final long MIN_INTERVAL_MS = 2000;

        private final Map<Long, Long> lastWrite = new java.util.concurrent.ConcurrentHashMap<>();
        private final Map<Long, Long> lastHash = new java.util.concurrent.ConcurrentHashMap<>();

        /** Written recently; capturing again should wait (the chunk stays queued) */
        public boolean coolingDown(long key, long now) {
            Long last = lastWrite.get(key);
            return last != null && now - last < MIN_INTERVAL_MS;
        }

        /** Captured at least once since the server started (it can't change while unloaded) */
        public boolean captured(long key) {
            return lastHash.containsKey(key);
        }

        /** Records a capture; false when the data is what's already on disk, so skip the write */
        public boolean changed(long key, byte[] data, long now) {
            java.util.zip.CRC32C crc = new java.util.zip.CRC32C();
            crc.update(data);
            long hash = ((long) data.length << 32) ^ crc.getValue();
            lastWrite.put(key, now);
            Long previous = lastHash.put(key, hash);
            return previous == null || previous != hash;
        }
    }

    /** Where live chunks go: the server folder, which is the server's working directory */
    public static Path liveDir() {
        return Path.of(System.getProperty("user.dir"), ".ingot", "live");
    }

    /** Writes a chunk's file, replacing it in one step so Ingot never reads half a file */
    public static void write(Path liveDir, String dimension, int chunkX, int chunkZ, byte[] data) throws IOException {
        String[] key = dimension.split(":", 2);
        Path dir = liveDir.resolve(key[0]).resolve(key[1]).resolve((chunkX >> 5) + "." + (chunkZ >> 5));
        Files.createDirectories(dir);
        Path file = dir.resolve(chunkX + "." + chunkZ + ".bin");
        Path tmp = dir.resolve(chunkX + "." + chunkZ + ".tmp");
        Files.write(tmp, data);
        Files.move(tmp, file, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
    }
}
