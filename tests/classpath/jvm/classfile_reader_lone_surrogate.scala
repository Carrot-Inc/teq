// jars: scala-library lonesurrogate-lib
// A class file's names and constants are to keep a lone UTF-16 unit as
// dotty's reader keeps it (`ClassfileParser`'s `getUTF`, Java's modified
// UTF-8): `J.S` is the unit `d83d`. J (tests/support/lone_surrogate) is
// compiled by javac and read from the class path. Queued as classfile-reader-names.
@main def main(): Unit =
  println(J.S.map(c => Integer.toHexString(c.toInt)).mkString(" "))
  println(J.T.map(c => Integer.toHexString(c.toInt)).mkString(" "))
  println(J.S == "\uD83D")
