#import <Foundation/Foundation.h>

// NSExtensionMain is provided by the macOS extension runtime, but it is not
// declared by the SDK headers used by this build.
extern int NSExtensionMain(int argc, const char *argv[]);

int main(int argc, const char * argv[]) {
  return NSExtensionMain(argc, argv);
}
