// expect: 9:58: error: type mismatch: found RawImage, required String | Int | Long | Double | Boolean
// expect: 1 error found
object Media:
  opaque type RawImage = String
  val step1: RawImage = "/step1.png"
object Attr:
  def set(value: String | Int | Long | Double | Boolean): String = value.toString
object Main:
  def main(args: Array[String]): Unit = println(Attr.set(Media.step1))
