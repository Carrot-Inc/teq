import component.widget.SegmentedSwitch

val rootVal = shared.describe("rootVal")

class RootClass:
  def member: String = shared.describe("rootMember")

@main def run(): Unit =
  println(SegmentedSwitch.segmentedSwitch(1))
  println(SegmentedSwitch.readMoreText)
  println(SegmentedSwitch.label)
  println(SegmentedSwitch.Parts.knob(2))
  println(component.widget.topLevel)
  println(rootVal)
  println(RootClass().member)
  println(shared.describe("main"))
