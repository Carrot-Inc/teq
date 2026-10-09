trait Show[T]:
  def show(t: T): String

object Fancy:
  given fancyInt: Show[Int] with
    def show(t: Int): String = s"fancy($t)"

object Plain:
  given plainInt: Show[Int] with
    def show(t: Int): String = s"plain($t)"

def render[T](t: T)(using s: Show[T]): String = s.show(t)

object Screen:
  given screenInt: Show[Int] with
    def show(t: Int): String = s"screen($t)"

  def own: String = render(1)

  def inDef: String =
    import Fancy.given
    render(2)

  def inBlock: String =
    val a = render(3)
    val b =
      import Plain.given
      render(4)
    a + " " + b

  def byName: String =
    import Fancy.fancyInt
    render(5)

  def inLambda: List[String] =
    List(6, 7).map: n =>
      import Plain.given
      render(n)

class Panel:
  import Fancy.given
  def draw: String = render(8)
  def nested: String =
    import Plain.given
    render(9)

@main def run(): Unit =
  println(Screen.own)
  println(Screen.inDef)
  println(Screen.inBlock)
  println(Screen.byName)
  println(Screen.inLambda)
  println(Panel().draw)
  println(Panel().nested)
