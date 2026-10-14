package xlib

object UiEnv:
  def helpers: Int = 2

final class FideUi(h: Int):
  def federation(n: Int): String = s"federation $n of $h"

final class PlayerUi(h: Int):
  def index: String = s"index of $h"
