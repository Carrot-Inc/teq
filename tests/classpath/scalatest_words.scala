// jars: scala-library scalatest-core scalatest-matchers-core scalatest-shouldmatchers scalatest-mustmatchers scalatest-compatible scalactic
// scalatest's `should` and `must` on a String: its two String receivers (`fullyMatch` and
// `compile`) are equally specific, so the argument decides among every alternative, and the
// generic `should[String](Matcher[String])` and `must[String, Equality](MatcherFactory1)` are
// the ones that take it.
import org.scalatest.matchers.should.Matchers.*
object Should:
  trait Named[A]:
    def name: String
  given Named[Int]:
    def name = "Int"
  def check(): Unit =
    val name: String = "Color"
    name should include("Col")
    summon[Named[Int]].name should include("In")
    name should not include ("Size")
    name shouldBe "Color"
    name should fullyMatch regex "C.*"
    name should startWith("C")
object Must extends org.scalatest.matchers.must.Matchers:
  def check(): Unit =
    val written = "ACTIVE"
    written must equal("ACTIVE")
    written must include("CT")
