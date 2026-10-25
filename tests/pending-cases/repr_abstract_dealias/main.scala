trait Abstract:
  type T <: String
object A extends Abstract
@main def abstractDealias(): Unit = println(MoreReflect.info[A.T])
