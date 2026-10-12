// jars: fixtures
// targets: js interp jvm
// An implicit class's constructor default from the jar's bodies: `new Applied[R]` reads the
// default on the companion that holds its getter, the module of the term name the class's
// conversion shares, not the conversion (zio's `ZLayer.scoped`).
import fix.icdef.IcDefaults

@main def main(): Unit =
  println(IcDefaults.applied[String](1))
  println(IcDefaults.counted.twice)
