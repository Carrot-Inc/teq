// A parent's argument is typed outside the class it is written on, for a class parent and a trait parent alike: a
// member the class declares does not shadow a name of the scope around, the constructor's parameters do, the
// class's givens are not searched, and `this` there is the class around.
object Test:
  val size: Int = 7
  val tag: String = "around"
  def pick(k: Int): String = "outer"
  given around: Int = 7
  class Base(val n: Int)
  class Titled(val title: String)
  trait Sized(val m: Int)
  trait Named(val name: String)
  object LoneClassParent extends Base(size) { val size: Int = 3 }
  object LoneTraitParent extends Sized(size) { val size: Int = 3 }
  class OwnMemberClassParent extends Base(size) { val size: Int = 3 }
  class OwnMemberTraitParent extends Sized(size) { val size: Int = 3 }
  class ParameterShadows(size: Int) extends Sized(size)
  class Overloaded extends Named(pick(1)) { def pick(k: Int): String = "own" }
  class ThisInClassParent extends Titled(this.tag) { val tag: String = "own" }
  class ThisInTraitParent extends Named(this.tag) { val tag: String = "own" }
  class GivenNotSeen extends Base(summon[Int]), Sized(summon[Int]) { given own: Int = 3 }
  def main(args: Array[String]): Unit =
    println(s"${LoneClassParent.n} ${LoneTraitParent.m} ${new OwnMemberClassParent().n} ${new OwnMemberTraitParent().m} ${new ParameterShadows(4).m}")
    println(s"${new Overloaded().name} ${new ThisInClassParent().title} ${new ThisInTraitParent().name}")
    val g = new GivenNotSeen()
    println(s"${g.n} ${g.m}")
