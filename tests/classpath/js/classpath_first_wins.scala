// jars: fixtures fixtures-shadow
// targets: js interp jvm
// Two jars define the class `fix.shadow.Shadowed`, the package object `shadow$package` and its given:
// the copy of the first jar on the class path is the one read, as scalac's class path gives it.
import fix.shadow.{Shadowed, shadowVersion, given}

@main def main(): Unit =
  println(Shadowed().which)
  println(shadowVersion)
  println(summon[Shadowed].which)
