// jars: unicode-lib
// targets: js interp jvm
// A jar's class named with a combining mark (U+0301 after the e of "Cafe", tests/support/unicode_lib.scala):
// its class file keeps the mark, a Java identifier part, as NameTransformer.encode does, which a
// lookup by the encoded name finds.
@main def run(): Unit =
  val c = new unicodelib.`Café`(7)
  println(c.n)
  println(c.twice)
