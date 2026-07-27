#import <Cocoa/Cocoa.h>
#import <FinderSync/FinderSync.h>

@interface CardinalFinderSync : FIFinderSync
@end

@implementation CardinalFinderSync

- (instancetype)init {
  self = [super init];
  if (self) {
    [FIFinderSyncController defaultController].directoryURLs =
        [NSSet setWithObject:[NSURL fileURLWithPath:@"/"]];
  }
  return self;
}

- (NSMenu *)menuForMenuKind:(FIMenuKind)menuKind {
  if (menuKind != FIMenuKindContextualMenuForItems) {
    return nil;
  }

  NSMenu *menu = [[NSMenu alloc] initWithTitle:@"Cardinal"];
  NSMenuItem *item = [[NSMenuItem alloc] initWithTitle:@"在 Cardinal 中搜索"
                                                 action:@selector(searchInCardinal:)
                                          keyEquivalent:@""];
  item.target = self;
  [menu addItem:item];
  return menu;
}

- (void)searchInCardinal:(id)sender {
  FIFinderSyncController *controller = [FIFinderSyncController defaultController];
  NSURL *selectedURL = controller.selectedItemURLs.firstObject;
  NSString *filename = selectedURL.lastPathComponent.stringByDeletingPathExtension;
  if (filename.length == 0) {
    return;
  }

  NSURLComponents *components = [[NSURLComponents alloc] init];
  components.scheme = @"cardinal";
  components.host = @"finder-search";
  components.queryItems = @[[NSURLQueryItem queryItemWithName:@"query" value:filename]];
  if (components.URL) {
    [[NSWorkspace sharedWorkspace] openURL:components.URL];
  }
}

@end
