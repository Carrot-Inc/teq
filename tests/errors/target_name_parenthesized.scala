import scala.annotation.targetName
def test(): Unit = {
  def clash = 1
  @targetName(("clash")) def other = 2
  ()
}
// scalac 3.8.4 reports the clash of the target name its parentheses hold, as it does of a bare literal.
// expect: 4:30: error: @targetName annotation "clash" clashes with other definition in same scope
// expect: 1 error found
