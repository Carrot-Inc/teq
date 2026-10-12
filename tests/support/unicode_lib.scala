// A class whose name holds a combining mark, U+0301 after the e of "Cafe": a Java identifier part,
// which scalac's NameTransformer.encode keeps as it is in the class file's name
// (tests/classpath/js/jar_unicode_name.scala).
package unicodelib

class `Café`(val n: Int):
  def twice: Int = n * 2
