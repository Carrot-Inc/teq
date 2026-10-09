import app.component.widget.SegmentedSwitch
import app.component.widget.SegmentedSwitch.Props
import app.util.Prelude.*
import app.util.Prelude.tw as utility

def show(c: Component): Unit =
  println(c.displayName + " (line " + c.line.toString + ")")
  println(render(c.render()))

val banner: Component = fc:
  div(cls := (tw"text-lg", Tw.empty, "banner"), Text("hello"))

@main def run(): Unit =
  println(tw"grid-cols-${1 + 2}" ++ Tw.empty ++ card)
  println(utility"mt-${4}")
  println(SegmentedSwitch.debugLabel)
  show(SegmentedSwitch.segmentedSwitch(Props("Dark mode", true, 4)))
  show(SegmentedSwitch.segmentedSwitch(Props("Compact", false, 1)))
  show(SegmentedSwitch.itemList(List(1, 2, 3)))
  show(banner)
  println(app.util.Prelude.useDebugLabel("qualified"))
