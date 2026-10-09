package demo.model

import demo.widgets.Widgets

/** A value the page holds in hook state across a hot swap. The package is a module of its own
  * (`demo.model.mjs`) and refers to the widgets as the widgets refer to it, the way an
  * application's packages refer to each other in cycles: a swap that ran this module again
  * would make `Theme`'s classes anew, and the value held would match no case of theirs. */
enum Theme:
  case Light
  case Dark(level: Int)

object Themes:
  def next(theme: Theme): Theme = theme match
    case Theme.Light => Theme.Dark(1)
    case Theme.Dark(level) => Theme.Dark(level + 1)

  def label(theme: Theme): String = Widgets.themePrefix + (theme match
    case Theme.Light => "light"
    case Theme.Dark(level) => s"dark $level")
