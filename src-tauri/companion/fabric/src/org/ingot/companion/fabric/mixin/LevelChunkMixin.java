package org.ingot.companion.fabric.mixin;

import net.minecraft.core.BlockPos;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.chunk.LevelChunk;
import org.ingot.companion.fabric.LiveTracker;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Every block change in a loaded chunk (players, water, pistons, explosions, growth...) */
@Mixin(LevelChunk.class)
abstract class LevelChunkMixin {
    @Inject(method = "setBlockState", at = @At("RETURN"))
    private void ingot$blockChanged(BlockPos pos, BlockState state, int flags, CallbackInfoReturnable<BlockState> cir) {
        // Returns the previous state, or null when nothing changed
        if (cir.getReturnValue() != null) {
            LiveTracker.blockChanged(((LevelChunk) (Object) this).getLevel(), pos);
        }
    }
}
