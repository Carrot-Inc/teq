package demo.model

import demo.widgets.Chip

object Chips:
  def isChip(value: Any): Boolean = value.isInstanceOf[Chip]
