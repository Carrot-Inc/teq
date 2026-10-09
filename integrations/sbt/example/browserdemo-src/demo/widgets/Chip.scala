package demo.widgets

import scala.scalajs.js

import demo.facade.React

/** A class of a per-file module that shared code tests (`demo.model.Chips.isChip`): a value
  * of it that the page holds would stop matching there after a swap of this file, so an edit
  * of this file reloads the page. */
final case class Chip(text: String)

object Chip:
  val first: Chip = Chip("chip: one")

  val View: js.Function0[js.Any] = () =>
    React.createElement("span", js.Dynamic.literal(id = "chip"), first.text)
