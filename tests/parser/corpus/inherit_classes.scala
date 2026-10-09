trait Priced:
  def price: Int
  def label: String = s"costs $price"

abstract class Item(val title: String, val year: Int = 2000) extends Priced:
  println(s"Item($title)")
  def kind: String
  val shelf: String = "general"
  lazy val code: String = { println(s"code of $title"); title.take(3).toUpperCase }
  def describe: String = s"$kind '$title' ($year) on $shelf, ${label}"
  final def id: String = s"$kind-$code"
  protected def secret: String = "s:" + title
  def reveal(other: Item): String = other.secret

class Book(title: String, val pages: Int) extends Item(title, 1999):
  println(s"Book($title)")
  def kind = "book"
  def price = pages / 10
  override val shelf = "fiction"
  override def label = "book " + super.label

class Comic(title: String) extends Book(title, 40):
  println(s"Comic($title)")
  override def kind = "comic"
  override lazy val code = "CMC"
  override def price = 3
  def both = secret + "/" + this.secret

case class Disc(name: String, tracks: Int) extends Item(name):
  def kind = "disc"
  val price = tracks * 2

object Catalogue extends Item("catalogue", 2024):
  def kind = "index"
  def price = 0
  override def toString = "Catalogue"

class Box[+T](val content: T):
  def get: T = content
  def show: String = s"Box($content)"

class IntBox extends Box[Int](7):
  def doubled: Int = get * 2

class Labelled[T](content: T, val tag: String) extends Box[T](content):
  override def show = tag + ":" + super.show

class Pair[A, B](a: A, b: B) extends Box[(A, B)]((a, b))

@main def run(): Unit =
  val items: List[Item] = List(Book("Dune", 600), Comic("Tintin"), Disc("Blue", 9), Catalogue)
  items.foreach(i => println(i.describe))
  items.foreach(i => println(i.id))
  println(items.head.reveal(items(1)))
  println(Comic("X").both)
  println(Disc("Blue", 9))
  println(Disc("Blue", 9) == Disc("Blue", 9))
  println(Disc("Blue", 9).copy(tracks = 1).describe)
  items.foreach {
    case c: Comic => println("comic " + c.pages)
    case b: Book => println("book " + b.pages)
    case Disc(n, t) => println(s"disc $n $t")
    case other => println("other " + other)
  }
  val any: Any = Comic("Y")
  println(any.isInstanceOf[Book])
  println(any.isInstanceOf[Item])
  println(any.isInstanceOf[Priced])
  println(any.isInstanceOf[Disc])
  println(any.asInstanceOf[Book].pages)
  val ib = IntBox()
  println(ib.doubled)
  println(ib.show)
  val boxes: List[Box[Any]] = List(ib, Labelled("x", "t"), Pair(1, "one"))
  boxes.foreach(b => println(b.show))
  println(Labelled(1.5, "d").get)
