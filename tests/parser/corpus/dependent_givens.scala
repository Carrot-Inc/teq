// A given whose type is a path-dependent member, found for the path the using clause names.
trait Codec:
  type Repr
  def show(r: Repr): String
object Upper extends Codec:
  type Repr = String
  def show(r: Repr): String = r.toUpperCase
object Count extends Codec:
  type Repr = Int
  def show(r: Repr): String = "#" * r

def render(c: Codec)(using r: c.Repr): String = c.show(r)
def viaSummon(c: Codec)(using c.Repr): String = c.show(summon[c.Repr])

trait Store:
  type Key
  def label(k: Key): String
object Names extends Store:
  type Key = String
  given defaultKey: Key = "anon"
  def label(k: Key): String = "name:" + k
def labelled(s: Store)(using k: s.Key): String = s.label(k)

def codecs(): Unit =
  given upper: Upper.Repr = "hello"
  given count: Count.Repr = 3
  println(render(Upper))
  println(render(Count))
  println(viaSummon(Upper))

def stores(): Unit =
  import Names.given
  println(labelled(Names))
  val store: Store = Names
  given storeKey: store.Key = store.asInstanceOf[Names.type].defaultKey.asInstanceOf[store.Key]
  println(labelled(store))
  println(labelled(Names)(using "given"))

@main def main(): Unit =
  codecs()
  stores()
