// jars: scala-library
// teq: --target jvm --std=scala-library
// command: build
// expect: error: the inline call exports.width in exports.viaTop was not expanded
// A top-level export of an inline val is an inline method, and `ValueOf` reads the value of the path
// `width.type` through it: a reference to the method that no expansion replaced, so the output would name a
// method nothing defines. scalac prints `3 3 3`.
package exports

object Source:
  inline val width = 3

export Source.width

def viaTop: Int = valueOf[width.type]

@main def run(): Unit = println(s"$viaTop ${valueOf[width.type]} $width")
