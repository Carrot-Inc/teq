/*
rules = ["file:../../support/ClassSymbols.scala"]
*/
// A Java class's static member class is named by the class it is written in, a type (`Point2D#Double#`), and so
// are a member of one and a member class two deep: the symbols scalac's SemanticDB has at these names.
package javasymbols

class Shapes: // assert: ClassSymbols.at
  def point: java.awt.geom.Point2D.Double = new java.awt.geom.Point2D.Double(1.0, 2.0) // assert: ClassSymbols.at
  def across(p: java.awt.geom.Point2D.Float): Float = p.x // assert: ClassSymbols.at
  def option: java.lang.invoke.MethodHandles.Lookup.ClassOption = null // assert: ClassSymbols.at
