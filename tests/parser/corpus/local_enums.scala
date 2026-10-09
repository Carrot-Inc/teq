// Enums defined in a block: entered as at the top level, with their companion, values and
// class cases, and used through a type parameter of the enclosing method.
object Main:
  def colors(pick: Int): String =
    enum Color:
      case Red, Green, Blue
      def isRed: Boolean = this == Red
    val c = Color.fromOrdinal(pick)
    val all = Color.values.map(_.toString).mkString(",")
    val named = c match
      case Color.Red => "r"
      case Color.Green => "g"
      case Color.Blue => "b"
    s"$c $named ${c.isRed} ${c.ordinal} $all ${Color.valueOf("Blue")}"

  def gallery[T](items: List[T]): String =
    enum Item:
      case Image(hash: T)
      case Uploading
      case AddImage
    val all: List[Item] = items.map(Item.Image(_)) ++ List(Item.Uploading, Item.AddImage)
    all.map {
      case Item.Image(h) => s"image($h)"
      case Item.Uploading => "uploading"
      case Item.AddImage => "add"
    }.mkString(" ")

  def parameterised(): String =
    enum Planet(val mass: Double):
      case Mercury extends Planet(0.33)
      case Earth extends Planet(5.97)
      def heavier: Boolean = mass > 1
    Planet.values.map(p => s"${p}:${p.heavier}").mkString(" ")

  def main(args: Array[String]): Unit =
    println(colors(0))
    println(colors(2))
    println(gallery(List("h1", "h2")))
    println(gallery(List(7)))
    println(parameterised())
