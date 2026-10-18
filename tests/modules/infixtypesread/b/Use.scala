package itrb

import itra.*

object Use:
  def pick(x: Int or String): Int = 1
  def both(x: Int Both String): Int = 2
  def flat(x: Int plain String): Int = 3
  val wrong: Int = "no"

trait More extends Members:
  def n(x: Int N String): Int
  def m(x: Int M String): Int
