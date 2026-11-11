trait HiddenSelfMembers {
  private def secret: Int = 1
  private val token: Int = 2
}
class SelfMemberReader { self: HiddenSelfMembers =>
  def readSecret: Int = secret
  def readToken: Int = token
}
// expect: 6:25: error: not found: secret
// expect: 7:24: error: not found: token
