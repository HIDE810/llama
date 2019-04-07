#include "screens.hpp"

#include <QImage>
#include <QPainter>
#include <QQmlProperty>
#include <QTimer>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickPaintedItem>

#include <cassert>
#include <utility>

Screen::Screen(WhichScreen screen, QQuickItem *parent):
        QQuickPaintedItem(parent),
        real_w(screen == WhichScreen::Top ? 400 : 320),
        real_h(240), screen(screen) {
    setOpaquePainting(true);
    setRenderTarget(QQuickPaintedItem::FramebufferObject);
}

void Screen::setCallbacks(Backend *backend, const FrontendCallbacks *callbacks) {
    this->backend = backend;
    this->callbacks = callbacks;
}

void Screen::paint(QPainter *painter) {
    painter->setRenderHint(QPainter::Antialiasing);

    size_t buf_size = 0;
    ColorFormat fmt;
    const uint8_t *buf;

    switch (screen) {
        case WhichScreen::Top: buf = callbacks->top_screen(backend, &buf_size, &fmt); break;
        case WhichScreen::Bottom: buf = callbacks->bot_screen(backend, &buf_size, &fmt); break;
        default:
            assert("Found improper screen!");
    }

    static const QMatrix SCREEN_ROTATE(0, -1,
                                       1,  0,
                                       0,  0);

    QImage::Format img_fmt;
    switch (fmt) {
    case COLOR_RGB8: img_fmt = QImage::Format_RGB888; break;
    case COLOR_RGBA8: img_fmt = QImage::Format_ARGB32; break;
    case COLOR_RGB565: img_fmt = QImage::Format_RGB16; break;
    case COLOR_RGB5A1: img_fmt = QImage::Format_RGB555; break; // Actually wrong
    case COLOR_RGBA4: img_fmt = QImage::Format_ARGB4444_Premultiplied; break;
    default:
        assert("Found improper color format!");
    }

    auto image = [&] {
        if (buf) {
            QImage img(buf, real_h, real_w, buf_size / real_w, img_fmt);
            return img.transformed(SCREEN_ROTATE);
        } else {
            return QImage();
        }
    };

    QRectF dst { 0, 0, (qreal)width(), (qreal)height() };
    painter->drawImage(dst, image());
}

void initScreenRepainter(QObject *scrn_view, Backend *backend, const FrontendCallbacks *callbacks) {
    {
        Screen *top_screen = qvariant_cast<Screen*>(QQmlProperty::read(scrn_view, "topScreen"));
        top_screen->setCallbacks(backend, callbacks);
        Screen *bot_screen = qvariant_cast<Screen*>(QQmlProperty::read(scrn_view, "botScreen"));
        bot_screen->setCallbacks(backend, callbacks);
    }

    QObject *top_screen = qvariant_cast<QObject*>(QQmlProperty::read(scrn_view, "topScreen"));
    QObject *bot_screen = qvariant_cast<QObject*>(QQmlProperty::read(scrn_view, "botScreen"));
    QTimer *scrn_update_timer = new QTimer(scrn_view);

    QObject::connect(scrn_update_timer, &QTimer::timeout, [=] {
        QMetaObject::invokeMethod(top_screen, "update");
        QMetaObject::invokeMethod(bot_screen, "update");
    });

    scrn_update_timer->start(16); // TODO: not ideal
}
