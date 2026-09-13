# Dolphin's badge for a cloud drive's files. KF6 ships no pkg-config files, so the headers and the one
# library it needs are named here.

MOC ?= /usr/lib/qt6/moc
CXXFLAGS ?= -O2
CXXFLAGS += -shared -fPIC -std=c++20 -DQT_NO_KEYWORDS
INCLUDES = $(shell pkg-config --cflags Qt6Core Qt6Network) -I/usr/include/KF6/KIOCore -I/usr/include/KF6/KIO -I/usr/include/KF6
LIBS = $(shell pkg-config --libs Qt6Core Qt6Network) -lKF6KIOCore
PLUGINS = $(HOME)/.local/lib/qt6/plugins/kf6/overlayicon

isle-overlay.so: main.cpp main.moc
	$(CXX) $(CXXFLAGS) $(INCLUDES) main.cpp -o $@ $(LIBS)

main.moc: main.cpp
	$(MOC) main.cpp -o main.moc

install: isle-overlay.so
	install -Dm755 isle-overlay.so $(PLUGINS)/isle-overlay.so

clean:
	rm -f isle-overlay.so main.moc

.PHONY: install clean
