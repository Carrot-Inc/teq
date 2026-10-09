// expect: 25:12: error: ambiguous given instances for Show[Int]: localInt, fancyInt
// expect: 30:12: error: ambiguous given instances for Show[Int]: plainInt, fancyInt
// expect: 35:31: error: ambiguous given instances for Show[Int]: fancyInt, plainInt
// expect: 48:13: error: no given instance of type Show[Long] was found for parameter s
// expect: 4 errors found

trait Show[T]:
  def show(t: T): String

object Fancy:
  given fancyInt: Show[Int] with
    def show(t: Int): String = s"fancy($t)"

object Plain:
  given plainInt: Show[Int] with
    def show(t: Int): String = s"plain($t)"

def render[T](t: T)(using s: Show[T]): String = s.show(t)

// An import ranks with the givens of the scope it stands in.

def importAfterGiven: String =
  given localInt: Show[Int] = Plain.plainInt
  import Fancy.given
  render(1)

def twoImports: String =
  import Fancy.given
  import Plain.plainInt
  render(2)

class Panel:
  import Fancy.given
  import Plain.given
  def draw: String = render(3)

object Shop:
  object Shows:
    private[Shop] given longShow: Show[Long] with
      def show(t: Long): String = s"long($t)"

  def inside: String =
    import Shows.given
    render(4L)

def outside: String =
  import Shop.Shows.given
  render(5L)
