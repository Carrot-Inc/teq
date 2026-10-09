//> using dep com.lihaoyi::sourcecode:0.4.2
// jars: sourcecode
package component.widget

import shared.Utils.{fc, named}
import shared.describe

object SegmentedSwitch:
  def segmentedSwitch[T](props: T): String = fc:
    "switch " + props

  val readMoreText: String = named("read more")

  // Typed while its sibling file has not been checked yet: the line has to come from there.
  def label: String = shared.inferredLabel + " / " + describe("label")

  object Parts:
    def knob(size: Int): String = fc:
      val doubled = size * 2
      "knob " + doubled

val topLevel = describe("topLevel")
