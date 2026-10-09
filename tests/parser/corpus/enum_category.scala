package model

import model.CatalogDisplayMode.Grid
import model.Models.*

enum CatalogDisplayMode:
  case List
  case DateGroupedList
  case Grid

trait CatalogCategory:
  def slug: String
  def shownName: String
  def emptyLabel: String
  def emptyDescription: Option[String]
  def displayMode: CatalogDisplayMode
  def newestFirst: Boolean
  def title: String = s"$shownName ($slug)"

object Models:
  enum WaiverCategory(val slug: String,
                         val shownName: String,
                         val emptyLabel: String,
                         val emptyDescription: Option[String],
                         val displayMode: CatalogDisplayMode,
                         val newestFirst: Boolean) extends CatalogCategory:
    case Active    extends WaiverCategory("active", "Active", "No active waivers", None, displayMode = CatalogDisplayMode.List, newestFirst = false)
    case Scheduled extends WaiverCategory("scheduled", "Scheduled", "No scheduled waivers", None, displayMode = CatalogDisplayMode.List, newestFirst = false)
    case All    extends WaiverCategory(
      "all",
      "All",
      "Automate Member Notices",
      Some("Create triggered notices, waivers and reminders that send automatically"),
      displayMode = Grid,
      newestFirst = true,
    )

    def isAll: Boolean = this == All

  enum MemberCategory(val slug: String, val shownName: String) extends CatalogCategory:
    case Everyone extends MemberCategory("all", "All")
    case Patron extends MemberCategory("patron", "PATRON")

    val emptyLabel: String = "No " + shownName.toLowerCase + " members"
    def emptyDescription: Option[String] = None
    def displayMode: CatalogDisplayMode = Grid
    val newestFirst: Boolean = slug == "patron"

def listing[CAT <: CatalogCategory](categories: Array[CAT], current: CAT): List[String] =
  categories.toList.map: c =>
    val marker = if c == current then "*" else " "
    s"$marker ${c.title} ${c.displayMode} ${c.newestFirst} ${c.emptyDescription.getOrElse(c.emptyLabel)}"

@main def main(): Unit =
  listing(WaiverCategory.values, WaiverCategory.All).foreach(println)
  listing(MemberCategory.values, MemberCategory.Patron).foreach(println)
  println(WaiverCategory.Active.isAll)
  println(WaiverCategory.All.isAll)
  val category: CatalogCategory = MemberCategory.Everyone
  println(category.slug)
  println(category.emptyLabel)
  println(category.newestFirst)
