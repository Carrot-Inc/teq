//> using options -deprecation:false --deprecation -Werror
object O { @deprecated("old", "1") def old(): Int = 1 }
@main def run(): Unit = println(O.old())
