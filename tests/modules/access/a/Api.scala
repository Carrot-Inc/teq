package aca

class Base:
  protected def hook: String = "base"
  private[aca] def internal: Int = 1
  def describe: String = hook + internal

object Friends:
  def peek(b: Base): Int = b.internal
