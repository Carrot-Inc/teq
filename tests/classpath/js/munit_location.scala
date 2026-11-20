// jars: munit munit-diff
// std: lean scala-library
//> using platform js
//> using jsVersion 1.22.0
//> using dep org.scalameta::munit::1.3.4
// munit's `Location` macro reads `java.io.File.separator` while it computes the source's path. Over scala-library
// the std's JavaScript layer is not loaded, so java.io.File is the JDK's class file: the interpreter's natives give
// its separators, as a JVM running the macro on this machine has them; the field was read as null, or refused.
def where(using loc: munit.Location): String = loc.path.split('/').last + ":" + loc.line

@main def run(): Unit = println(where)
