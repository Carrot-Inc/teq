trait Evidence[A]:
  def value: Int
trait Root:
  given Evidence[Int] with
    def value = 42
trait Left0 extends Root
trait Right0 extends Root
trait Left1 extends Left0 with Right0
trait Right1 extends Left0 with Right0
trait Left2 extends Left1 with Right1
trait Right2 extends Left1 with Right1
trait Left3 extends Left2 with Right2
trait Right3 extends Left2 with Right2
trait Left4 extends Left3 with Right3
trait Right4 extends Left3 with Right3
trait Left5 extends Left4 with Right4
trait Right5 extends Left4 with Right4
trait Left6 extends Left5 with Right5
trait Right6 extends Left5 with Right5
trait Left7 extends Left6 with Right6
trait Right7 extends Left6 with Right6
trait Left8 extends Left7 with Right7
trait Right8 extends Left7 with Right7
trait Left9 extends Left8 with Right8
trait Right9 extends Left8 with Right8
trait Left10 extends Left9 with Right9
trait Right10 extends Left9 with Right9
trait Left11 extends Left10 with Right10
trait Right11 extends Left10 with Right10
trait Left12 extends Left11 with Right11
trait Right12 extends Left11 with Right11
trait Left13 extends Left12 with Right12
trait Right13 extends Left12 with Right12
trait Left14 extends Left13 with Right13
trait Right14 extends Left13 with Right13
object Instances extends Left14 with Right14
import Instances.given
@main def run(): Unit = println(summon[Evidence[Int]].value)
