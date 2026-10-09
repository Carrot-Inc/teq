// jars: scala-library abi-callbacks-lib
// std: scala-library
// targets: jvm
// The JVM target's ABI: scalac-compiled code (tests/support/abi_callbacks_lib.scala,
// built by scalac 3.8.4 into the jar abi-callbacks-lib) calls back into teq's classes: FunctionN
// through its specialised entry points, Product members, hashing, a companion's apply/unapply
// and a static forwarder found by reflection. Greeter is named by a string only, which link
// mode keeps as scalac would.
import abi.Lib

case class Point(x: Int, y: Int)
object Greeter:
  def greeting: String = "hello from Greeter"

object Main:
  def check(what: String)(body: => Any): Unit =
    try println(s"$what: ${body}")
    catch case e: Throwable => println(s"$what: ${e.getClass.getName}")
  def main(args: Array[String]): Unit =
    check("Function1 through apply$mcII$sp")(Lib.applyTwice(_ + 1, 5))
    check("Function2 through apply$mcIII$sp")(Lib.applyBoth(_ * _))
    check("Function1.andThen")(Lib.compose(_ + 1, _ * 10)(2))
    check("Product members")(Lib.describe(Point(1, 2)))
    check("productElementNames")(Lib.names(Point(1, 2)))
    check("canEqual")(Lib.canEq(Point(1, 2), Point(3, 4)))
    check("hash value")(Lib.hash(Point(1, 2)))
    check("equals and hash")(Lib.sameAs(Point(1, 2), Point(1, 2)))
    check("companion apply and unapply by reflection")(Lib.viaCompanion("Point", 7))
    check("static forwarder")(Lib.viaStaticForwarder("Greeter"))
    check("Function0 in a Seq")(Lib.runAll(Seq(() => "a", () => "b")))
    check("Ordering")(Lib.sortDesc(Seq(2, 3, 1), Ordering.Int))
    check("by-name default of a jar method")(abi.Defaults.clue(1))
    check("its getter ran at each use")(abi.Defaults.evaluated)
    check("by-name argument written")(abi.Defaults.clue(2, "given"))
    check("by-name default alone")(abi.Defaults.twice())
    check("by-name default of an instance method over the arguments before it")(abi.Tagged("t").label(3)())
