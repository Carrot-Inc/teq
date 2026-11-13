// Library classes whose parents are JDK classes the standard library does not declare: a library class's
// parents are read when the class is completed, the JDK's among them.
package jdkparent

class Gate(parties: Int) extends java.util.concurrent.Phaser(parties)
class Listed extends javax.management.AttributeList
class Spot(x: Int, y: Int) extends java.awt.Point(x, y)
class Spot2(x: Int, y: Int) extends java.awt.Point(x, y)
class Press(t: String) extends javax.swing.JButton(t):
  def label: String = "press:" + t
