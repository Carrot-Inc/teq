// A class file's names and constants keep a lone UTF-16 unit as dotty's reader keeps it (`ClassfileParser`'s
// `getUTF`, Java's modified UTF-8): `J.S` is the unit `d83d`. teq's class-file reader decodes the entry with
// `String::from_utf16_lossy`, so the constant reads as U+FFFD (table (c)'s note on the M2e refold). J.java is compiled
// by javac and put on both compilers' class paths. Queued as classfile-reader-names (the coordinator's ruling of 28 Oct).
@main def main(): Unit =
  println(J.S.map(c => Integer.toHexString(c.toInt)).mkString(" "))
  println(J.T.map(c => Integer.toHexString(c.toInt)).mkString(" "))
  println(J.S == "\uD83D")
