// The part of java.awt Indigo's plugin names outside its font generator, for bench/indigo/overlay/generate.scala:
// the colour `RGB.toColor` makes, which only the plugin's FontGen draws with, replaced by fontgen.scala. teq's
// std has no java.awt.
package java.awt

final class Color(r: Int, g: Int, b: Int):
  def getRed: Int = r
  def getGreen: Int = g
  def getBlue: Int = b
  def getRGB: Int = 0xff000000 | (r & 0xff) << 16 | (g & 0xff) << 8 | (b & 0xff)
