package logic

import ui.{Mark, Selection}
import views.Label

def isSelection(x: Any): Boolean = x.isInstanceOf[Selection]

def index(x: Any): Int = x match
  case Selection(i) => i
  case _ => -1

def name(mark: Mark): String = mark match
  case Mark.Red => "red"
  case Mark.Green => "green"

def isRed(mark: Mark): Boolean = mark == Mark.Red

def label(text: String): String = views.heading(new Label(text).shown) + " of " + views.title
