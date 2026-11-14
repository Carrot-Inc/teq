package jm

// A static member class of the JDK constructed: the constructor's used name is the companion's
// (`java;awt;geom;Point2D$;Double;init;`), as zinc records it over scalac.
class Made:
  def point: java.awt.geom.Point2D.Double = new java.awt.geom.Point2D.Double()
