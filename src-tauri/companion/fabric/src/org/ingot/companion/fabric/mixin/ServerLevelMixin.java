package org.ingot.companion.fabric.mixin;

import java.util.function.BooleanSupplier;
import net.minecraft.server.level.ServerLevel;
import org.ingot.companion.fabric.LiveTracker;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Runs the live map's work once per world tick, on the server thread */
@Mixin(ServerLevel.class)
abstract class ServerLevelMixin {
    @Inject(method = "tick", at = @At("TAIL"))
    private void ingot$tick(BooleanSupplier hasTimeLeft, CallbackInfo ci) {
        LiveTracker.tick((ServerLevel) (Object) this);
    }
}
