# JNA and the UniFFI bindings find native symbols and callbacks by reflection.
-keep class com.sun.jna.** { *; }
-keep class * extends com.sun.jna.** { *; }
-keep class org.mobileofficefree.engine.** { *; }
-dontwarn java.awt.**
