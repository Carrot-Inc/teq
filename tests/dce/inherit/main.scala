package dce

abstract class Vehicle(val wheels: Int):
  println(s"vehicle with $wheels wheels")
  def sound: String = "rumble"
  def unusedVehicleMethod: String = "never"
  def describe: String = s"$wheels wheels going $sound"

trait Electric:
  val charge: Int = 80
  def sound: String
  def hum: String = "hum " + charge
  def unusedTraitMethod: String = "never"

class Car extends Vehicle(4):
  override def sound: String = "vroom and " + super.sound

class Tram extends Vehicle(8) with Electric:
  override def sound: String = hum

class NeverBuilt extends Vehicle(2):
  override def sound: String = "neverBuiltSound"

abstract class NeverExtendedBase:
  def lonely: String = "lonelyMember"

@main def run(): Unit =
  val fleet: List[Vehicle] = List(Car(), Tram())
  fleet.foreach(v => println(v.describe))
