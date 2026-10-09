// A given of an intersection type provides each of its parts (cats' `Invariant.catsInstancesForList:
// Monad[List] & Alternative[List] & CoflatMap[List]`), through the companions in the parts' scope.
trait Base[F[_]]:
  def name: String
trait TC1[F[_]] extends Base[F]:
  def one: String
trait TC2[F[_]] extends Base[F]:
  def two: String
trait TC3[F[_]] extends Base[F]:
  def three: String
object Base:
  given both: (TC1[List] & TC2[List]) & TC3[List] = new TC1[List] with TC2[List] with TC3[List]:
    def name = "list"
    def one = "one"
    def two = "two"
    def three = "three"
  given TC2[Option] with
    def name = "option"
    def two = "opt-two"
object Main:
  def main(args: Array[String]): Unit =
    println(summon[TC2[List]].two)
    println(summon[TC3[List]].three)
    println(summon[TC1[List]].one)
    println(summon[Base[List]].name)
    println(summon[TC2[Option]].two)
