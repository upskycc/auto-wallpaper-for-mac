#include <Availability.h>
#include <CoreFoundation/CoreFoundation.h>
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

static int display_is_on(io_service_t service) {
    CFNumberRef num = IORegistryEntryCreateCFProperty(
        service, CFSTR("CurrentPowerState"), kCFAllocatorDefault, 0);
    if (!num) {
        return 1;
    }
    int value = 4;
    CFNumberGetValue(num, kCFNumberIntType, &value);
    CFRelease(num);
    return value >= 3;
}

struct WaitCtx {
    CFRunLoopRef loop;
    io_service_t service;
    int display_on;
};

static void interest(
    void *refcon,
    io_service_t service,
    uint32_t message_type,
    void *message_argument
) {
    struct WaitCtx *ctx = (struct WaitCtx *)refcon;
    (void)service;
    (void)message_type;
    (void)message_argument;
    if (display_is_on(ctx->service)) {
        ctx->display_on = 1;
        CFRunLoopStop(ctx->loop);
    }
}

int wallflow_wait_until_display_on(void) {
    io_service_t service = IOServiceGetMatchingService(
        wallflow_iokit_port(), IOServiceNameMatching("IODisplayWrangler"));
    if (!service) {
        return 1;
    }
    if (display_is_on(service)) {
        IOObjectRelease(service);
        return 1;
    }

    IONotificationPortRef port = IONotificationPortCreate(wallflow_iokit_port());
    if (!port) {
        IOObjectRelease(service);
        return 1;
    }

    struct WaitCtx ctx;
    ctx.loop = CFRunLoopGetCurrent();
    ctx.service = service;
    ctx.display_on = 0;

    io_object_t notifier = IO_OBJECT_NULL;
    kern_return_t kr = IOServiceAddInterestNotification(
        port,
        service,
        kIOGeneralInterest,
        interest,
        &ctx,
        &notifier);
    if (kr != KERN_SUCCESS) {
        IONotificationPortDestroy(port);
        IOObjectRelease(service);
        return 1;
    }

    CFRunLoopSourceRef src = IONotificationPortGetRunLoopSource(port);
    CFRunLoopAddSource(ctx.loop, src, kCFRunLoopDefaultMode);
    CFRunLoopRun();
    CFRunLoopRemoveSource(ctx.loop, src, kCFRunLoopDefaultMode);

    if (notifier) {
        IOObjectRelease(notifier);
    }
    IONotificationPortDestroy(port);
    IOObjectRelease(service);
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
