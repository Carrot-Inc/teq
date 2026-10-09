package pma

sealed trait S
case object A extends S
case object B extends S

trait Holder:
  type M <: S
