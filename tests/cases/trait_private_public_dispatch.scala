trait PublicSelfMember {
  def value: Int = 7
}
trait HiddenSelfMember {
  private def value: Int = 9
}
class SelfMemberChoice extends PublicSelfMember { self: HiddenSelfMember =>
  private def own: Int = 3
  def result: Int = value + own
}
@main def run(): Unit = println((new SelfMemberChoice with HiddenSelfMember).result)
