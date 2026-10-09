package app.icons

import util.Prelude.IconSet.*
import util.Prelude.{Tw as Style, twice, clampInt as bounded, joined}

def iconLine: String = home + " " + icon("x").name

def styled: Style = Style(List("s", "t"))

def four: Int = bounded(twice(2), 0, 10)

def numbers: String = List(1, 2).joined(_.toString)
