// jars: fixtures
// targets: js interp
// An object named as a package object is (`Token$package`) in the fixtures jar, which an inline
// body of the jar names: read as the object, not as its package.
@main def run(): Unit =
  println(fix.ponapi.PonApi.ref)
