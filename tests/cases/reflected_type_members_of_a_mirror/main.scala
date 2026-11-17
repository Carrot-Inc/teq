import scala.deriving.Mirror

case class Parcel(weight: Int)
case object Depot

object Main {
  def main(args: Array[String]): Unit = {
    println(Members.has[Parcel.type]("MirroredType"))
    println(Members.has[Parcel.type]("MirroredElemTypes"))
    println(Members.has[Parcel.type]("MirroredLabel"))
    println(Members.has[Mirror.Product]("MirroredType"))
    println(Members.has[Mirror.Sum]("MirroredElemTypes"))
    println(Members.has[Depot.type]("MirroredType"))
    println(Members.typeNames[Mirror])
    println(Members.typeNames[Mirror.Product])
    println(Members.typeNames[Mirror.Sum])
    println(Members.typeNames[Mirror.Singleton])
    println(Members.typeNames[Mirror.SingletonProxy])
  }
}
