#include <Availability.h>
#include <CoreFoundation/CoreFoundation.h>
#include <CoreGraphics/CoreGraphics.h>
#include <IOKit/IOKitLib.h>
#include <IOKit/ps/IOPowerSources.h>
#include <IOKit/ps/IOPSKeys.h>
#include <stdint.h>

static mach_port_t wallflow_iokit_port(void) {
    mach_port_t port = MACH_PORT_NULL;
#if defined(__MAC_OS_X_VERSION_MAX_ALLOWED) && __MAC_OS_X_VERSION_MAX_ALLOWED >= 120000
    if (__builtin_available(macOS 12.0, *)) {
        if (IOMainPort(MACH_PORT_NULL, &port) == KERN_SUCCESS) {
            return port;
        }
        return MACH_PORT_NULL;
    }
#endif
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    if (IOMasterPort(MACH_PORT_NULL, &port) != KERN_SUCCESS) {
        port = kIOMasterPortDefault;
    }
#pragma clang diagnostic pop
    return port;
}

static int wrangler_is_on(io_service_t service) {
    CFNumberRef num = IORegistryEntryCreateCFProperty(
        service, CFSTR("CurrentPowerState"), kCFAllocatorDefault, 0);
    if (!num) {
        return -1;
    }
    int value = 0;
    CFNumberGetValue(num, kCFNumberIntType, &value);
    CFRelease(num);
    return value >= 3;
}

int wallflow_display_is_on(void) {
    uint32_t count = 0;
    if (CGGetOnlineDisplayList(0, NULL, &count) == kCGErrorSuccess) {
        if (count == 0) {
            return 0;
        }
        CGDirectDisplayID ids[32];
        uint32_t n = 0;
        if (CGGetOnlineDisplayList(32, ids, &n) == kCGErrorSuccess && n > 0) {
            for (uint32_t i = 0; i < n; i++) {
                if (CGDisplayIsActive(ids[i]) && !CGDisplayIsAsleep(ids[i])) {
                    return 1;
                }
            }
            return 0;
        }
    }

    io_service_t service = IOServiceGetMatchingService(
        wallflow_iokit_port(), IOServiceNameMatching("IODisplayWrangler"));
    if (service) {
        int on = wrangler_is_on(service);
        IOObjectRelease(service);
        if (on >= 0) {
            return on;
        }
    }
    return 1;
}

struct WaitCtx {
    CFRunLoopRef loop;
    io_service_t wrangler;
    int display_on;
};

static void stop_if_on(struct WaitCtx *ctx) {
    if (wallflow_display_is_on()) {
        ctx->display_on = 1;
        CFRunLoopStop(ctx->loop);
    }
}

static void interest(
    void *refcon,
    io_service_t service,
    uint32_t message_type,
    void *message_argument
) {
    (void)service;
    (void)message_type;
    (void)message_argument;
    stop_if_on((struct WaitCtx *)refcon);
}

static void display_reconfig(
    CGDirectDisplayID display,
    CGDisplayChangeSummaryFlags flags,
    void *user_info
) {
    (void)display;
    (void)flags;
    stop_if_on((struct WaitCtx *)user_info);
}

int wallflow_wait_until_display_on(void) {
    if (wallflow_display_is_on()) {
        return 1;
    }

    struct WaitCtx ctx;
    ctx.loop = CFRunLoopGetCurrent();
    ctx.wrangler = IO_OBJECT_NULL;
    ctx.display_on = 0;

    IONotificationPortRef note_port = IONotificationPortCreate(wallflow_iokit_port());
    io_object_t wrangler_note = IO_OBJECT_NULL;
    io_object_t root_note = IO_OBJECT_NULL;
    CFRunLoopSourceRef src = NULL;

    if (note_port) {
        src = IONotificationPortGetRunLoopSource(note_port);
        if (src) {
            CFRunLoopAddSource(ctx.loop, src, kCFRunLoopDefaultMode);
        }
        ctx.wrangler = IOServiceGetMatchingService(
            wallflow_iokit_port(), IOServiceNameMatching("IODisplayWrangler"));
        if (ctx.wrangler) {
            IOServiceAddInterestNotification(
                note_port, ctx.wrangler, kIOGeneralInterest, interest, &ctx, &wrangler_note);
        }
        io_service_t root = IOServiceGetMatchingService(
            wallflow_iokit_port(), IOServiceMatching("IOPMrootDomain"));
        if (root) {
            IOServiceAddInterestNotification(
                note_port, root, kIOGeneralInterest, interest, &ctx, &root_note);
            IOObjectRelease(root);
        }
    }

    CGDisplayRegisterReconfigurationCallback(display_reconfig, &ctx);

    while (!wallflow_display_is_on()) {
        CFRunLoopRun();
    }
    ctx.display_on = 1;

    CGDisplayRemoveReconfigurationCallback(display_reconfig, &ctx);
    if (wrangler_note) {
        IOObjectRelease(wrangler_note);
    }
    if (root_note) {
        IOObjectRelease(root_note);
    }
    if (src) {
        CFRunLoopRemoveSource(ctx.loop, src, kCFRunLoopDefaultMode);
    }
    if (note_port) {
        IONotificationPortDestroy(note_port);
    }
    if (ctx.wrangler) {
        IOObjectRelease(ctx.wrangler);
    }
    return ctx.display_on;
}

int wallflow_on_ac_power(void) {
    CFTypeRef blob = IOPSCopyPowerSourcesInfo();
    if (!blob) {
        return 1;
    }
    CFStringRef type = IOPSGetProvidingPowerSourceType(blob);
    int ac = type && CFStringCompare(type, CFSTR(kIOPMACPowerKey), 0) == kCFCompareEqualTo;
    CFRelease(blob);
    return ac;
}

static void power_changed(void *context) {
    (void)context;
    CFRunLoopStop(CFRunLoopGetCurrent());
}

int wallflow_wait_until_ac_power(void) {
    if (wallflow_on_ac_power()) {
        return 1;
    }
    CFRunLoopSourceRef src = IOPSNotificationCreateRunLoopSource(power_changed, NULL);
    if (!src) {
        return 1;
    }
    CFRunLoopRef loop = CFRunLoopGetCurrent();
    CFRunLoopAddSource(loop, src, kCFRunLoopDefaultMode);
    while (!wallflow_on_ac_power()) {
        CFRunLoopRun();
    }
    CFRunLoopRemoveSource(loop, src, kCFRunLoopDefaultMode);
    CFRelease(src);
    return 1;
}
