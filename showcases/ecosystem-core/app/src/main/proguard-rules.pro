# ProGuard/R8 rules for the AngNative Android shell.
#
# JNI resolves native entry points by exported symbol name and resolves the
# onNative* / buildLayoutRequest instance methods on JniBridge with GetMethodID.
# Keep that class whole so neither set of members can be renamed or removed.
-keep class com.angnative.app.JniBridge { *; }

# The launcher activity is instantiated reflectively by Android; R8 keeps
# manifest-referenced classes, but keep it whole as an explicit safeguard.
-keep class com.angnative.app.EcosystemCoreActivity { *; }
