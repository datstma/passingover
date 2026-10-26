package se.mansby.passingover

import android.appwidget.AppWidgetManager
import android.content.Context
import android.content.SharedPreferences
import android.view.View
import android.widget.RemoteViews
import es.antonborri.home_widget.HomeWidgetLaunchIntent
import es.antonborri.home_widget.HomeWidgetProvider

/**
 * The home-screen widget (M6, D40): the nearest flight the app saw last, as
 * of when it saw it. Tapping it opens the app, which looks again.
 */
class PassingOverWidget : HomeWidgetProvider() {
    override fun onUpdate(
        context: Context,
        appWidgetManager: AppWidgetManager,
        appWidgetIds: IntArray,
        widgetData: SharedPreferences,
    ) {
        for (id in appWidgetIds) {
            val views = RemoteViews(context.packageName, R.layout.passing_over_widget)
            views.setTextViewText(
                R.id.widget_title,
                widgetData.getString("title", null) ?: context.getString(R.string.app_name),
            )
            views.setTextViewText(
                R.id.widget_destination,
                widgetData.getString("destination", null)
                    ?: context.getString(R.string.widget_open_app),
            )
            views.setOrHide(R.id.widget_details, widgetData.getString("details", null))
            views.setOrHide(R.id.widget_updated, widgetData.getString("updated", null))
            views.setOnClickPendingIntent(
                R.id.widget_root,
                HomeWidgetLaunchIntent.getActivity(context, MainActivity::class.java),
            )
            appWidgetManager.updateAppWidget(id, views)
        }
    }
}

private fun RemoteViews.setOrHide(id: Int, text: String?) {
    if (text.isNullOrEmpty()) {
        setViewVisibility(id, View.GONE)
    } else {
        setTextViewText(id, text)
        setViewVisibility(id, View.VISIBLE)
    }
}
