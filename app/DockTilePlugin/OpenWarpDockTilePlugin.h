#import <Cocoa/Cocoa.h>
#import <Foundation/Foundation.h>

@interface OpenWarpDockTilePlugIn : NSObject <NSDockTilePlugIn>
{
    id iconChangedObserver;
    id defaultsObserver;
}

@property(strong) id iconChangedObserver;
@property(strong) id defaultsObserver;
@end
