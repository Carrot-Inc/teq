package app.component.widget

import app.util.Prelude.{*, given}

object SegmentedSwitch:
  final case class Props(label: String, active: Boolean, size: Int)

  val debugLabel: String = useDebugLabel("switch")

  def segmentedSwitch(props: Props): Component = fc:
    val width = tw"w-${props.size}"
    div(
      cls := (tw"flex items-center", "font-bold" -> props.active, width -> (props.size > 2), card),
      title := props.label,
      style := (css"width: ${props.size * 10}px", css"opacity: ${if props.active then 1 else 0.5}"),
      span(cls := "label", Text(props.label))
    )

  def itemList(items: List[Int]): Component =
    val label = useDebugLabel("items")
    fc:
      div(
        cls := (pad(2), label),
        items.toKeyedNodes(_ * 10)(i => li(cls := (("item", i > 1)), Text(i.toString)))
      )
