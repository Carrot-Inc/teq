// A top-level case class's companion as indigo's package aliases name it: a singleton type of its path and a
// val of it (`val Texture: shared.materials.Texture.type = shared.materials.Texture`), the companion
// `Desugar.classDef` gives every case class, which teq makes where a path or a value first names it.
package app

val Texture: shared.materials.Texture.type = shared.materials.Texture
val Mouse = shared.materials.Mouse

@main def run(): Unit =
  println(Texture("a", 1))
  println(Mouse(3))
  println(List(("c", 3)).map(Texture.apply.tupled))
  println(Texture)
