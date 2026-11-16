// teq: --deprecation --werror --wconf msg=Support.for.trait.App:s
// `App`'s deprecation warning carries scala-library's message: a build that silences it by that text passes
// under `--werror`.
object Test extends App {
  println("silenced by its text")
}
