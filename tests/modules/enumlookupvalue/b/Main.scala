package enumlookupvalue

object Main:
  def main(args: Array[String]): Unit =
    val names = List("Low", "High")
    val ordinals = List(0)
    val unknown = List("Mid")
    println(Readers.all(names))
    println(Readers.byOrdinal(1))
    println(Readers.inlined(names))
    println(Readers.shown(names))
    println(Readers.shapes(ordinals))
    println(Readers.viaInline(names))
    println(Readers.viaTransparent(names))
    val read: String => Tone = Tone.valueOf
    println(read("Low"))
    println(List(0, 1).map(Tone.fromOrdinal))
    println(try Readers.shown(unknown) catch case e: IllegalArgumentException => "no such case")
