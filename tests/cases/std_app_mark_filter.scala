// teq: --deprecation --werror --wconf msg=Support.for.trait.App:s
// `App`'s deprecation warning carries scala-library's message: a build that silences it by that text passes
// under `--werror`. The deprecated definition of the program's own is what makes the build check deprecations.
object Test extends App {
  @deprecated("use nothing", "1.0") def spare(): Unit = ()
  println("silenced by its text")
}
