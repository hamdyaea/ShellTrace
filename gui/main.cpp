#include <QApplication>
#include <QColor>
#include <QDebug>
#include <QCloseEvent>
#include <QComboBox>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QFrame>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QLabel>
#include <QIcon>
#include <QPainter>
#include <QPainterPath>
#include <QPalette>
#include <QSettings>
#include <QLineEdit>
#include <QLocale>
#include <QMainWindow>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProcess>
#include <QProgressBar>
#include <QPushButton>
#include <QStandardPaths>
#include <QStatusBar>
#include <QStyleFactory>
#include <QTabWidget>
#include <QTableWidget>
#include <QTemporaryDir>
#include <QTextCursor>
#include <QVBoxLayout>
#include <QWidget>
#include <cmath>

namespace {
// Native raster rendering of the selected shield emblem. It needs only Qt6
// Widgets, even if a distribution does not ship the Qt SVG image plugin.
QIcon badgeIcon() {
    QPixmap pixmap(256, 256);
    pixmap.fill(Qt::transparent);
    QPainter p(&pixmap);
    p.setRenderHint(QPainter::Antialiasing);
    QPainterPath shield;
    shield.moveTo(128, 10);
    shield.lineTo(224, 45);
    shield.quadTo(237, 50, 237, 66);
    shield.lineTo(237, 145);
    shield.quadTo(236, 202, 128, 245);
    shield.quadTo(20, 202, 19, 145);
    shield.lineTo(19, 66);
    shield.quadTo(19, 50, 32, 45);
    shield.closeSubpath();
    QLinearGradient edge(10, 25, 244, 240);
    edge.setColorAt(0, QColor("#16E5F7"));
    edge.setColorAt(0.55, QColor("#118EFF"));
    edge.setColorAt(1, QColor("#173DE9"));
    p.fillPath(shield, edge);
    QPainterPath inner;
    inner.moveTo(128, 28);
    inner.lineTo(209, 57);
    inner.quadTo(219, 60, 219, 74);
    inner.lineTo(219, 142);
    inner.quadTo(218, 191, 128, 228);
    inner.quadTo(38, 191, 37, 142);
    inner.lineTo(37, 73);
    inner.quadTo(37, 60, 47, 57);
    inner.closeSubpath();
    p.fillPath(inner, QColor("#071729"));
    p.setPen(QPen(QColor("#F7FAFF"), 9, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    p.drawLine(QPointF(61, 75), QPointF(83, 91));
    p.drawLine(QPointF(83, 91), QPointF(61, 107));
    p.drawLine(QPointF(91, 108), QPointF(117, 108));
    p.setBrush(QColor("#05291F"));
    p.setPen(QPen(QColor("#31F68B"), 9));
    p.drawEllipse(QPointF(171, 90), 36, 36);
    p.setPen(QPen(QColor("#31F68B"), 5, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    QPainterPath pulse;
    pulse.moveTo(145, 92); pulse.lineTo(157, 92); pulse.lineTo(163, 79);
    pulse.lineTo(170, 105); pulse.lineTo(178, 87); pulse.lineTo(183, 92);
    pulse.lineTo(196, 92);
    p.drawPath(pulse);
    p.setPen(QPen(QColor("#31F68B"), 12, Qt::SolidLine, Qt::RoundCap));
    p.drawLine(QPointF(194, 114), QPointF(215, 136));
    p.setPen(QPen(QColor("#1DF08D"), 4));
    p.drawLine(62, 134, 62, 205);
    p.setPen(Qt::NoPen);
    p.setBrush(QColor("#28F491"));
    for (int y : {145, 172, 199}) p.drawEllipse(QPointF(62, y), 7, 7);
    p.setBrush(QColor("#25334C"));
    for (int y : {132, 159, 186}) p.drawRoundedRect(QRectF(78, y, 122, 24), 12, 12);
    p.setPen(QColor("#F6FAFF"));
    QFont mono("Monospace", 12, QFont::DemiBold);
    mono.setStyleHint(QFont::TypeWriter);
    p.setFont(mono);
    p.drawText(QRectF(89, 132, 112, 24), Qt::AlignVCenter, "execve(…)");
    p.drawText(QRectF(89, 159, 112, 24), Qt::AlignVCenter, "open(…)");
    p.drawText(QRectF(89, 186, 112, 24), Qt::AlignVCenter, "write(…)");
    p.end();
    return QIcon(pixmap);
}

QString locateEngine() {
    const QString dir = QCoreApplication::applicationDirPath();
    const QStringList candidates = {
        // User-local and system installations place both binaries together.
        dir + "/shelltrace",
        dir + "/../../target/release/shelltrace",
        dir + "/../target/release/shelltrace",
        QDir::currentPath() + "/target/release/shelltrace",
        QStandardPaths::findExecutable("shelltrace")
    };
    for (const auto &candidate : candidates) {
        if (!candidate.isEmpty() && QFileInfo(candidate).isExecutable()) {
            return QFileInfo(candidate).canonicalFilePath();
        }
    }
    return {};
}

QString field(const QJsonObject &object, const QString &key) {
    const auto value = object.value(key);
    if (value.isString()) return value.toString();
    if (value.isBool()) return value.toBool() ? QStringLiteral("true") : QStringLiteral("false");
    if (value.isDouble()) return QString::number(value.toDouble(), 'f', 0);
    if (value.isArray()) {
        QStringList parts;
        for (const auto &entry : value.toArray()) parts << entry.toString();
        return parts.join(", ");
    }
    return QStringLiteral("—");
}

QString fileState(const QJsonValue &value) {
    if (!value.isObject()) return QStringLiteral("—");
    const QJsonObject state = value.toObject();
    QString result = state.value("file_type").toString() + " · "
        + QString::number(state.value("size_bytes").toDouble()) + " B";
    if (state.contains("mode")) {
        result += " · " + QString::number(state.value("mode").toInt(), 8).rightJustified(4, '0');
    }
    const QString hash = state.value("sha256").toString();
    if (!hash.isEmpty()) result += " · SHA-256 " + hash.left(14) + "…";
    const QString target = state.value("symlink_target").toString();
    if (!target.isEmpty()) result += " → " + target;
    return result;
}
}

class ShellTraceWindow final : public QMainWindow {
public:
    ShellTraceWindow() : proc_(this), french_(QLocale::system().language() == QLocale::French),
        systemPalette_(QApplication::palette()) {
        setWindowTitle("ShellTrace · Linux impact inspector");
        setWindowIcon(badgeIcon());
        resize(1120, 820);
        setMinimumSize(780, 610);

        auto *root = new QWidget(this);
        root->setObjectName("shelltrace-root");
        auto *layout = new QVBoxLayout(root);
        layout->setContentsMargins(22, 18, 22, 18);
        layout->setSpacing(13);

        auto *heading = new QHBoxLayout;
        auto *brandIcon = new QLabel(root);
        brandIcon->setPixmap(badgeIcon().pixmap(62, 62));
        heading->addWidget(brandIcon);
        auto *headText = new QVBoxLayout;
        auto *brand = new QLabel("ShellTrace", root);
        brand->setObjectName("brand");
        auto *subtitle = new QLabel(L("Understand what your Linux command changes.",
                                      "Comprenez ce que votre commande Linux modifie."), root);
        subtitle->setObjectName("subtitle");
        headText->addWidget(brand);
        headText->addWidget(subtitle);
        heading->addLayout(headText, 1);
        auto *version = new QLabel("RUST ENGINE  ·  QT6 DESKTOP", root);
        version->setObjectName("eyebrow");
        heading->addWidget(version);
        theme_ = new QComboBox(root);
        theme_->setObjectName("appearance-selector");
        theme_->setAccessibleName(L("Appearance", "Apparence"));
        theme_->setToolTip(L("Appearance: Dark, Light or System. Dark is the default.",
                             "Apparence : Sombre, Clair ou Système. Sombre par défaut."));
        theme_->addItem(L("Dark", "Sombre"), "dark");
        theme_->addItem(L("Light", "Clair"), "light");
        theme_->addItem(L("System", "Système"), "system");
        const QString savedTheme = QSettings().value("appearance/theme", "dark").toString();
        const int index = theme_->findData(savedTheme);
        theme_->setCurrentIndex(index >= 0 ? index : 0);
        heading->addSpacing(12);
        heading->addWidget(theme_);
        layout->addLayout(heading);

        auto *config = new QGroupBox(L("New analysis", "Nouvelle analyse"), root);
        auto *form = new QGridLayout(config);
        form->setHorizontalSpacing(12);
        form->setVerticalSpacing(9);
        int r = 0;

        mode_ = new QComboBox(config);
        mode_->addItem(L("Sandbox · recommended", "Sandbox · recommandé"));
        mode_->addItem(L("Host · executes on your computer", "Hôte · exécute sur votre ordinateur"));
        form->addWidget(new QLabel(L("Execution mode", "Mode d'exécution"), config), r, 0);
        form->addWidget(mode_, r++, 1, 1, 3);

        executable_ = new QLineEdit("/bin/sh", config);
        auto *browseExe = new QPushButton(L("Browse", "Parcourir"), config);
        form->addWidget(new QLabel(L("Executable", "Exécutable"), config), r, 0);
        form->addWidget(executable_, r, 1, 1, 2);
        form->addWidget(browseExe, r++, 3);

        arguments_ = new QLineEdit("-c \"echo ShellTrace > gui-demo.txt\"", config);
        arguments_->setPlaceholderText(L("Arguments, e.g. -c \"echo hello > demo.txt\"",
                                          "Arguments, ex. -c \"echo bonjour > demo.txt\""));
        form->addWidget(new QLabel(L("Arguments", "Arguments"), config), r, 0);
        form->addWidget(arguments_, r++, 1, 1, 3);

        scope_ = new QLineEdit(QDir::tempPath() + "/shelltrace-gui-demo", config);
        QDir().mkpath(scope_->text());
        auto *browseScope = new QPushButton(L("Folder", "Dossier"), config);
        form->addWidget(new QLabel(L("Watch folder (host)", "Dossier surveillé (hôte)"), config), r, 0);
        form->addWidget(scope_, r, 1, 1, 2);
        form->addWidget(browseScope, r++, 3);

        seed_ = new QLineEdit(config);
        seed_->setPlaceholderText(L("Optional: copy a folder into the sandbox", "Facultatif : copier un dossier dans le sandbox"));
        auto *browseSeed = new QPushButton(L("Folder", "Dossier"), config);
        form->addWidget(new QLabel(L("Sandbox input", "Dossier source du sandbox"), config), r, 0);
        form->addWidget(seed_, r, 1, 1, 2);
        form->addWidget(browseSeed, r++, 3);

        engine_ = new QLineEdit(locateEngine(), config);
        auto *browseEngine = new QPushButton(L("Binary", "Binaire"), config);
        engine_->setPlaceholderText(L("Path to built shelltrace executable", "Chemin vers le binaire shelltrace"));
        form->addWidget(new QLabel(L("Rust engine", "Moteur Rust"), config), r, 0);
        form->addWidget(engine_, r, 1, 1, 2);
        form->addWidget(browseEngine, r++, 3);

        form->setColumnStretch(1, 1);
        form->setColumnStretch(2, 1);
        layout->addWidget(config);

        warning_ = new QLabel(root);
        warning_->setObjectName("notice");
        warning_->setWordWrap(true);
        layout->addWidget(warning_);

        auto *buttons = new QHBoxLayout;
        run_ = new QPushButton(L("Start analysis", "Démarrer l'analyse"), root);
        run_->setObjectName("primary");
        stop_ = new QPushButton(L("Stop", "Arrêter"), root);
        stop_->setEnabled(false);
        open_ = new QPushButton(L("Open JSON report", "Ouvrir un rapport JSON"), root);
        save_ = new QPushButton(L("Save report…", "Enregistrer le rapport…"), root);
        save_->setEnabled(false);
        buttons->addWidget(run_);
        buttons->addWidget(stop_);
        buttons->addStretch();
        buttons->addWidget(open_);
        buttons->addWidget(save_);
        layout->addLayout(buttons);

        auto *metrics = new QHBoxLayout;
        metrics->setSpacing(10);
        metrics->addWidget(metricCard(L("Filesystem changes", "Fichiers modifiés"), fileMetric_), 1);
        metrics->addWidget(metricCard(L("Command executions", "Exécutions"), processMetric_), 1);
        metrics->addWidget(metricCard(L("Internet socket calls", "Appels réseau Internet"), networkMetric_), 1);
        metrics->addWidget(metricCard(L("Sensitive paths", "Chemins sensibles"), sensitiveMetric_), 1);
        layout->addLayout(metrics);

        auto *filters = new QHBoxLayout;
        search_ = new QLineEdit(root);
        search_->setPlaceholderText(L("Search results by path, syscall, address…", "Rechercher chemin, appel système, adresse…"));
        origin_ = new QComboBox(root);
        origin_->addItem(L("All event origins", "Toutes les origines"), "all");
        origin_->addItem(L("Command only", "Commande uniquement"), "command");
        origin_->addItem(L("Bubblewrap setup", "Préparation Bubblewrap"), "sandbox_setup");
        origin_->addItem(L("Unclassified", "Non classé"), "unknown");
        filters->addWidget(search_, 1);
        filters->addWidget(origin_);
        layout->addLayout(filters);

        tabs_ = new QTabWidget(root);
        files_ = makeTable({L("Change", "Changement"), L("Path", "Chemin"),
                            L("Before", "Avant"), L("After", "Après"), L("Evidence", "Preuve")});
        operations_ = makeTable({L("Origin", "Origine"), "PID", L("Operation", "Opération"),
                                L("Paths", "Chemins"), L("Result", "Résultat")});
        processes_ = makeTable({L("Origin", "Origine"), "PID", L("Operation", "Opération"),
                               L("Executable", "Exécutable"), L("Result", "Résultat")});
        network_ = makeTable({L("Origin", "Origine"), "PID", L("Operation", "Opération"),
                             L("Type", "Type"), L("Address", "Adresse"), L("Result", "Résultat")});
        sensitive_ = makeTable({L("Origin", "Origine"), "PID", L("Operation", "Opération"),
                               L("Sensitive path", "Chemin sensible"), L("Result", "Résultat")});
        tabs_->addTab(files_, L("File changes", "Fichiers"));
        tabs_->addTab(operations_, L("File operations", "Opérations"));
        tabs_->addTab(processes_, L("Processes", "Processus"));
        tabs_->addTab(network_, L("Network", "Réseau"));
        tabs_->addTab(sensitive_, L("Sensitive paths", "Chemins sensibles"));
        log_ = new QPlainTextEdit(root);
        log_->setReadOnly(true);
        log_->setPlaceholderText(L("Command output appears here.", "La sortie de la commande apparaît ici."));
        json_ = new QPlainTextEdit(root);
        json_->setReadOnly(true);
        tabs_->addTab(log_, L("Execution log", "Journal"));
        tabs_->addTab(json_, "JSON");
        layout->addWidget(tabs_, 1);

        progress_ = new QProgressBar(root);
        progress_->setTextVisible(false);
        progress_->setMaximumHeight(5);
        progress_->setRange(0, 1);
        progress_->setValue(0);
        layout->addWidget(progress_);
        setCentralWidget(root);
        statusBar()->showMessage(L("Ready · sandbox is selected by default.", "Prêt · le sandbox est sélectionné par défaut."));

        connect(mode_, &QComboBox::currentIndexChanged, this, [this](int) { updateMode(); });
        connect(theme_, &QComboBox::currentIndexChanged, this, [this](int) { applyTheme(); });
        connect(browseExe, &QPushButton::clicked, this, [this] {
            const QString f = QFileDialog::getOpenFileName(this, L("Choose executable", "Choisir l'exécutable"));
            if (!f.isEmpty()) executable_->setText(f);
        });
        connect(browseEngine, &QPushButton::clicked, this, [this] {
            const QString f = QFileDialog::getOpenFileName(this, L("Choose ShellTrace binary", "Choisir le binaire ShellTrace"));
            if (!f.isEmpty()) engine_->setText(f);
        });
        connect(browseScope, &QPushButton::clicked, this, [this] {
            const QString f = QFileDialog::getExistingDirectory(this, L("Choose watched folder", "Dossier à surveiller"), scope_->text());
            if (!f.isEmpty()) scope_->setText(f);
        });
        connect(browseSeed, &QPushButton::clicked, this, [this] {
            const QString f = QFileDialog::getExistingDirectory(this, L("Choose sandbox input", "Dossier source du sandbox"));
            if (!f.isEmpty()) seed_->setText(f);
        });
        connect(run_, &QPushButton::clicked, this, [this] { startAnalysis(); });
        connect(stop_, &QPushButton::clicked, this, [this] {
            if (proc_.state() != QProcess::NotRunning) {
                proc_.kill();  // Terminates the tracer; detached grandchildren may remain.
                log_->appendPlainText(L("Stop requested. Background descendants may survive.", "Arrêt demandé. Des processus descendants peuvent subsister."));
            }
        });
        connect(open_, &QPushButton::clicked, this, [this] {
            const QString f = QFileDialog::getOpenFileName(this, L("Open JSON report", "Ouvrir un rapport JSON"),
                                                             QDir::homePath(), "JSON (*.json)");
            if (!f.isEmpty()) loadReport(f);
        });
        connect(save_, &QPushButton::clicked, this, [this] { saveReport(); });
        connect(search_, &QLineEdit::textChanged, this, [this] { filterRows(); });
        connect(origin_, &QComboBox::currentIndexChanged, this, [this](int) { filterRows(); });
        connect(&proc_, &QProcess::readyReadStandardOutput, this, [this] {
            appendOutput(QString::fromLocal8Bit(proc_.readAllStandardOutput()));
        });
        connect(&proc_, &QProcess::readyReadStandardError, this, [this] {
            appendOutput(QString::fromLocal8Bit(proc_.readAllStandardError()));
        });
        connect(&proc_, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
            if (error == QProcess::FailedToStart) {
                setRunning(false);
                QMessageBox::critical(this, "ShellTrace", L("Could not launch the Rust engine.", "Impossible de démarrer le moteur Rust.")
                    + "\n" + proc_.errorString());
            }
        });
        connect(&proc_, &QProcess::finished, this, [this](int code, QProcess::ExitStatus status) {
            appendOutput(QString::fromLocal8Bit(proc_.readAllStandardOutput()));
            appendOutput(QString::fromLocal8Bit(proc_.readAllStandardError()));
            setRunning(false);
            if (QFileInfo::exists(reportFile_)) {
                loadReport(reportFile_);
            } else {
                statusBar()->showMessage(L("Command finished without a JSON report.", "Commande terminée sans rapport JSON."));
            }
            const bool okay = status == QProcess::NormalExit && code == 0;
            statusBar()->showMessage(okay
                ? L("Analysis completed. Review the results and warnings.", "Analyse terminée. Consultez les résultats et avertissements.")
                : L("Execution failed or was stopped (exit ", "Exécution échouée ou arrêtée (code ") + QString::number(code) + ").");
        });
        applyTheme();
        updateMode();
    }

private:
    void applyTheme() {
        const QString choice = theme_->currentData().toString();
        const bool dark = choice == "dark";
        const bool light = choice == "light";
        QPalette palette = systemPalette_;
        QString qss;
        if (dark || light) {
            const QColor window(dark ? "#111827" : "#F3F6FA");
            const QColor surface(dark ? "#1D293A" : "#FFFFFF");
            const QColor alternate(dark ? "#26364B" : "#ECF2F8");
            const QColor foreground(dark ? "#F4F8FF" : "#142235");
            const QColor secondary(dark ? "#CAD7E8" : "#4E6075");
            const QColor border(dark ? "#526780" : "#BFCCD9");
            const QColor accent(dark ? "#8BD8FF" : "#1263A7");
            const QColor selection(dark ? "#265D98" : "#CBDFF4");
            const QColor selectedText(dark ? "#FFFFFF" : "#142235");
            const QColor hover(dark ? "#354A65" : "#DDE8F4");
            const QColor primary(dark ? "#155D9D" : "#135E9E");
            const QColor primaryHover(dark ? "#174D84" : "#114C80");

            // Use both an explicit Qt palette (for native dialogs) and
            // concrete QSS colors (for Fusion/KDE/GTK style engines).
            palette.setColor(QPalette::Window, window);
            palette.setColor(QPalette::WindowText, foreground);
            palette.setColor(QPalette::Base, surface);
            palette.setColor(QPalette::AlternateBase, alternate);
            palette.setColor(QPalette::Text, foreground);
            palette.setColor(QPalette::Button, alternate);
            palette.setColor(QPalette::ButtonText, foreground);
            palette.setColor(QPalette::ToolTipBase, surface);
            palette.setColor(QPalette::ToolTipText, foreground);
            palette.setColor(QPalette::Mid, secondary);
            palette.setColor(QPalette::Midlight, border);
            palette.setColor(QPalette::Highlight, selection);
            palette.setColor(QPalette::HighlightedText, selectedText);
            palette.setColor(QPalette::Link, accent);
            palette.setColor(QPalette::PlaceholderText, secondary);
            palette.setColor(QPalette::BrightText, foreground);
            for (auto role : {QPalette::WindowText, QPalette::Text, QPalette::ButtonText}) {
                palette.setColor(QPalette::Disabled, role, secondary);
            }
            palette.setColor(QPalette::Disabled, QPalette::Base, alternate);

            qss = QString::fromUtf8(R"QSS(
                QMainWindow, QWidget#shelltrace-root {
                    background-color: @WINDOW@; color: @FG@;
                }
                QLabel { color: @FG@; background: transparent; }
                QLabel#brand { color: @FG@; font-size: 29px; font-weight: 750; }
                QLabel#subtitle { color: @MUTED@; font-size: 12px; }
                QLabel#eyebrow { color: @ACCENT@; font-size: 10px; font-weight: 700; }
                QLabel#notice {
                    color: @FG@; background-color: @ALT@;
                    padding: 10px 13px; border-radius: 8px;
                    border: 1px solid @BORDER@;
                }
                QLabel#metric-value { color: @ACCENT@; font-size: 22px; font-weight: bold; }
                QLabel#metric-label { color: @MUTED@; font-size: 11px; }
                QFrame#metric {
                    background-color: @SURFACE@; border: 1px solid @BORDER@;
                    border-radius: 9px;
                }
                QGroupBox {
                    color: @FG@; font-weight: 650; border: 1px solid @BORDER@;
                    border-radius: 7px; margin-top: 13px; padding-top: 10px;
                }
                QGroupBox::title {
                    color: @FG@; subcontrol-origin: margin;
                    subcontrol-position: top left; left: 12px; padding: 0 4px;
                }
                QLineEdit, QPlainTextEdit, QComboBox, QTableWidget,
                QTableView, QAbstractItemView, QMenu, QListView {
                    background-color: @SURFACE@;
                    alternate-background-color: @ALT@;
                    color: @FG@; border: 1px solid @BORDER@;
                    border-radius: 5px;
                    selection-background-color: @SELECTION@;
                    selection-color: @SELECTED_FG@;
                }
                QLineEdit, QComboBox { padding: 6px 9px; min-height: 21px; }
                QPlainTextEdit, QTableWidget { padding: 4px; }
                QLineEdit:focus, QComboBox:focus, QPlainTextEdit:focus {
                    border: 2px solid @ACCENT@;
                }
                QLineEdit:disabled, QComboBox:disabled {
                    color: @MUTED@; background-color: @ALT@;
                }
                QComboBox QAbstractItemView {
                    background-color: @SURFACE@; color: @FG@;
                    alternate-background-color: @ALT@;
                    selection-background-color: @SELECTION@;
                    selection-color: @SELECTED_FG@;
                    border: 1px solid @BORDER@;
                }
                QComboBox::drop-down { border: none; width: 22px; }
                QPushButton {
                    color: @FG@; background-color: @ALT@;
                    border: 1px solid @BORDER@; border-radius: 6px;
                    padding: 7px 13px; min-height: 22px;
                }
                QPushButton:hover { color: @FG@; background-color: @HOVER@; }
                QPushButton:focus { border: 2px solid @ACCENT@; }
                QPushButton:disabled {
                    color: @MUTED@; background-color: @ALT@;
                }
                QPushButton#primary {
                    background-color: @PRIMARY@; color: #FFFFFF;
                    border: 1px solid @PRIMARY@; font-weight: bold;
                    padding: 8px 19px;
                }
                QPushButton#primary:hover {
                    background-color: @PRIMARY_HOVER@; color: #FFFFFF;
                }
                QPushButton#primary:disabled {
                    background-color: @ALT@; color: @MUTED@;
                    border: 1px solid @BORDER@;
                }
                QTabWidget::pane {
                    background-color: @SURFACE@; border: 1px solid @BORDER@;
                }
                QTabBar::tab {
                    color: @FG@; background-color: @ALT@;
                    border: 1px solid @BORDER@; padding: 9px 13px;
                }
                QTabBar::tab:selected {
                    color: @FG@; background-color: @SURFACE@;
                    border-bottom: 2px solid @ACCENT@;
                }
                QTabBar::tab:hover:!selected { background-color: @HOVER@; }
                QHeaderView::section {
                    color: @FG@; background-color: @ALT@;
                    border: 1px solid @BORDER@; padding: 7px;
                }
                QTableWidget::item { color: @FG@; }
                QTableWidget::item:selected {
                    color: @SELECTED_FG@; background-color: @SELECTION@;
                }
                QToolTip {
                    color: @FG@; background-color: @SURFACE@;
                    border: 1px solid @BORDER@; padding: 5px;
                }
                QStatusBar {
                    color: @FG@; background-color: @WINDOW@;
                    border-top: 1px solid @BORDER@;
                }
                QProgressBar {
                    color: @FG@; background-color: @ALT@;
                    border: 1px solid @BORDER@; border-radius: 3px;
                }
                QProgressBar::chunk { background-color: @ACCENT@; }
            )QSS");
            const QList<QPair<QString, QString>> values = {
                {"@WINDOW@", window.name()}, {"@SURFACE@", surface.name()},
                {"@ALT@", alternate.name()}, {"@FG@", foreground.name()},
                {"@MUTED@", secondary.name()}, {"@BORDER@", border.name()},
                {"@ACCENT@", accent.name()}, {"@SELECTION@", selection.name()},
                {"@SELECTED_FG@", selectedText.name()}, {"@HOVER@", hover.name()},
                {"@PRIMARY@", primary.name()}, {"@PRIMARY_HOVER@", primaryHover.name()}
            };
            for (const auto &value : values) qss.replace(value.first, value.second);
        } else {
            // No dark/light color override at all for the System choice.
            qss = QString::fromUtf8(R"QSS(
                QLabel#brand { font-size: 29px; font-weight: 750; }
                QLabel#subtitle { font-size: 12px; }
                QLabel#eyebrow { font-size: 10px; font-weight: 700; }
                QLabel#notice { padding: 10px 13px; border-radius: 8px; }
                QLabel#metric-value { font-size: 22px; font-weight: bold; }
                QLabel#metric-label { font-size: 11px; }
                QFrame#metric { border-radius: 9px; }
                QGroupBox { font-weight: 650; padding-top: 10px; }
                QLineEdit { padding: 5px 7px; }
                QTabBar::tab { padding: 8px 13px; }
                QPushButton#primary { font-weight: bold; padding: 8px 19px; }
            )QSS");
        }

        // Clear the stale QSS before resetting the palette, otherwise a
        // switch may leave black text on already-dark popups or controls.
        QApplication::setStyleSheet(QString());
        QApplication::setPalette(palette);
        QApplication::setStyleSheet(qss);
        QSettings().setValue("appearance/theme", choice);
    }

    QString L(const char *en, const char *fr) const {
        return QString::fromUtf8(french_ ? fr : en);
    }

    QFrame *metricCard(const QString &caption, QLabel *&value) {
        auto *frame = new QFrame(this);
        frame->setObjectName("metric");
        auto *col = new QVBoxLayout(frame);
        col->setContentsMargins(13, 11, 13, 11);
        col->setSpacing(1);
        value = new QLabel("—", frame);
        value->setObjectName("metric-value");
        auto *label = new QLabel(caption, frame);
        label->setObjectName("metric-label");
        label->setWordWrap(true);
        col->addWidget(value);
        col->addWidget(label);
        return frame;
    }

    QTableWidget *makeTable(const QStringList &headers) {
        auto *table = new QTableWidget(this);
        table->setColumnCount(headers.size());
        table->setHorizontalHeaderLabels(headers);
        table->setEditTriggers(QAbstractItemView::NoEditTriggers);
        table->setSelectionBehavior(QAbstractItemView::SelectRows);
        table->setSelectionMode(QAbstractItemView::SingleSelection);
        table->setAlternatingRowColors(true);
        table->setWordWrap(false);
        table->verticalHeader()->setVisible(false);
        table->horizontalHeader()->setStretchLastSection(true);
        table->horizontalHeader()->setSectionResizeMode(QHeaderView::Interactive);
        table->setSortingEnabled(true);
        connect(table, &QTableWidget::cellDoubleClicked, this, [this, table](int row, int) {
            QStringList values;
            for (int col = 0; col < table->columnCount(); ++col) {
                if (auto *item = table->item(row, col)) {
                    values << table->horizontalHeaderItem(col)->text() + ": " + item->text();
                }
            }
            QMessageBox::information(this, L("Event details", "Détails"), values.join("\n"));
        });
        return table;
    }

    void insertRow(QTableWidget *table, const QStringList &values) {
        const int row = table->rowCount();
        table->insertRow(row);
        for (int col = 0; col < values.size() && col < table->columnCount(); ++col) {
            auto *item = new QTableWidgetItem(values[col]);
            item->setToolTip(values[col]);
            table->setItem(row, col, item);
        }
    }

    void updateMode() {
        const bool sandbox = mode_->currentIndex() == 0;
        scope_->setEnabled(!sandbox);
        seed_->setEnabled(sandbox);
        warning_->setText(sandbox
            ? L("Sandbox: host root is read-only, but this is NOT a secure malware sandbox. The temporary workspace is analyzed after execution.",
                "Sandbox : système hôte en lecture seule, mais ce n'est PAS une isolation sécurisée contre les malwares. L'espace temporaire est analysé après exécution.")
            : L("HOST MODE: the command can modify your computer and contact networks. Only the chosen folder is compared before and after.",
                "MODE HÔTE : la commande peut modifier votre ordinateur et utiliser le réseau. Seul le dossier choisi est comparé avant/après."));
    }

    void setRunning(bool running) {
        run_->setEnabled(!running);
        stop_->setEnabled(running);
        open_->setEnabled(!running);
        progress_->setRange(0, running ? 0 : 1);
        if (!running) progress_->setValue(0);
    }

    void appendOutput(const QString &chunk) {
        if (chunk.isEmpty()) return;
        // Do not allow unlimited output to exhaust the GUI's memory.
        if (log_->toPlainText().size() > 250000) {
            if (!logTruncated_) {
                log_->appendPlainText(L("[Output truncated. JSON report is unaffected.]", "[Sortie tronquée. Le JSON n'est pas affecté.]"));
                logTruncated_ = true;
            }
            return;
        }
        log_->moveCursor(QTextCursor::End);
        log_->insertPlainText(chunk.left(100000));
        log_->moveCursor(QTextCursor::End);
    }

    void startAnalysis() {
        if (proc_.state() != QProcess::NotRunning) return;
        const QString binary = engine_->text().trimmed();
        if (binary.isEmpty() || !QFileInfo(binary).isExecutable()) {
            QMessageBox::warning(this, "ShellTrace", L("Build ShellTrace first and choose its executable.",
                "Compilez ShellTrace puis sélectionnez son exécutable."));
            return;
        }
        const QString executable = executable_->text().trimmed();
        if (executable.isEmpty()) {
            QMessageBox::warning(this, "ShellTrace", L("Select a command to execute.", "Sélectionnez une commande."));
            return;
        }
        const bool sandbox = mode_->currentIndex() == 0;
        if (!sandbox && !QDir(scope_->text()).exists()) {
            QMessageBox::warning(this, "ShellTrace", L("The watched folder does not exist.", "Le dossier surveillé n'existe pas."));
            return;
        }
        if (sandbox && !seed_->text().trimmed().isEmpty() && !QDir(seed_->text().trimmed()).exists()) {
            QMessageBox::warning(this, "ShellTrace", L("The sandbox input folder does not exist.", "Le dossier source du sandbox n'existe pas."));
            return;
        }
        if (!sandbox) {
            const auto answer = QMessageBox::warning(this, L("Confirm real host execution", "Confirmer l'exécution réelle"),
                L("This command will run on your real computer with your privileges. It may modify or delete files OUTSIDE the watched folder.\n\nContinue?",
                  "Cette commande sera exécutée sur votre ordinateur avec vos droits. Elle peut modifier ou supprimer des fichiers HORS du dossier surveillé.\n\nContinuer ?"),
                QMessageBox::Yes | QMessageBox::Cancel, QMessageBox::Cancel);
            if (answer != QMessageBox::Yes) return;
        }
        if (!temp_.isValid()) {
            QMessageBox::critical(this, "ShellTrace", L("Unable to create temporary report directory.", "Impossible de créer le dossier temporaire de rapport."));
            return;
        }
        reportFile_ = temp_.path() + "/report.json";
        QFile::remove(reportFile_);
        for (auto *table : {files_, operations_, processes_, network_, sensitive_}) table->setRowCount(0);
        fileMetric_->setText("—");
        processMetric_->setText("—");
        networkMetric_->setText("—");
        sensitiveMetric_->setText("—");
        save_->setEnabled(false);
        json_->clear();
        log_->clear();
        logTruncated_ = false;
        QStringList args;
        args << (sandbox ? "sandbox" : "run");
        if (!sandbox) args << "--scope" << scope_->text();
        if (sandbox && !seed_->text().trimmed().isEmpty()) args << "--seed" << seed_->text().trimmed();
        args << "--json" << reportFile_ << "--no-color" << "--" << executable;
        args << QProcess::splitCommand(arguments_->text());
        proc_.setWorkingDirectory(sandbox ? QDir::currentPath() : scope_->text());
        proc_.setProgram(binary);
        proc_.setArguments(args);
        proc_.setProcessChannelMode(QProcess::SeparateChannels);
        setRunning(true);
        statusBar()->showMessage(L("Analyzing…", "Analyse en cours…"));
        proc_.start();
    }

    void loadReport(const QString &path) {
        QFile file(path);
        if (!file.open(QIODevice::ReadOnly)) {
            QMessageBox::warning(this, "ShellTrace", L("Unable to read report.", "Impossible de lire le rapport.") + " " + file.errorString());
            return;
        }
        if (file.size() > 25 * 1024 * 1024) {
            QMessageBox::warning(this, "ShellTrace", L("The report exceeds 25 MiB.", "Le rapport dépasse 25 Mio."));
            return;
        }
        const QByteArray bytes = file.readAll();
        QJsonParseError error;
        const auto doc = QJsonDocument::fromJson(bytes, &error);
        if (error.error != QJsonParseError::NoError || !doc.isObject()) {
            QMessageBox::warning(this, "ShellTrace", L("Invalid JSON report.", "Rapport JSON invalide.") + " " + error.errorString());
            return;
        }
        const QJsonObject report = doc.object();
        if (!report.contains("filesystem_changes") || !report.contains("observations")) {
            QMessageBox::warning(this, "ShellTrace", L("Not a ShellTrace report.", "Ce fichier n'est pas un rapport ShellTrace."));
            return;
        }
        reportFile_ = path;
        json_->setPlainText(QString::fromUtf8(doc.toJson(QJsonDocument::Indented)));
        for (auto *table : {files_, operations_, processes_, network_, sensitive_}) {
            table->setSortingEnabled(false);
            table->setRowCount(0);
        }
        const QJsonArray changes = report.value("filesystem_changes").toArray();
        for (const auto &entry : changes) {
            const QJsonObject o = entry.toObject();
            insertRow(files_, {field(o, "kind"), field(o, "path"),
                        fileState(o.value("before")), fileState(o.value("after")), field(o, "evidence")});
        }
        const QJsonObject obs = report.value("observations").toObject();
        auto fill = [this, &obs](QTableWidget *table, const QString &key, const QStringList &columns) {
            for (const auto &entry : obs.value(key).toArray()) {
                const QJsonObject object = entry.toObject();
                QStringList values;
                for (const auto &col : columns) values << field(object, col);
                insertRow(table, values);
            }
        };
        fill(operations_, "filesystem_operations", {"origin", "pid", "syscall", "paths", "result"});
        fill(processes_, "process_events", {"origin", "pid", "syscall", "executable", "result"});
        fill(network_, "network_events", {"origin", "pid", "syscall", "category", "address", "result"});
        fill(sensitive_, "sensitive_accesses", {"origin", "pid", "syscall", "path", "result"});
        for (auto *table : {files_, operations_, processes_, network_, sensitive_}) {
            table->setSortingEnabled(true);
            table->resizeColumnsToContents();
            table->horizontalHeader()->setStretchLastSection(true);
        }
        fileMetric_->setText(QString::number(changes.size()));
        const auto pe = obs.value("process_events").toArray();
        int executed = 0, internet = 0, sensitive = 0;
        for (const auto &value : pe) {
            const auto obj = value.toObject();
            if (obj.value("origin").toString() == "command" && !obj.value("executable").isNull()) ++executed;
        }
        for (const auto &value : obs.value("network_events").toArray()) {
            const auto obj = value.toObject();
            if (obj.value("origin").toString() == "command" && obj.value("category").toString() == "internet") ++internet;
        }
        for (const auto &value : obs.value("sensitive_accesses").toArray()) {
            if (value.toObject().value("origin").toString() == "command") ++sensitive;
        }
        processMetric_->setText(QString::number(executed));
        networkMetric_->setText(QString::number(internet));
        sensitiveMetric_->setText(QString::number(sensitive));
        const QString mode = report.value("mode").toString();
        const QString date = QDateTime::fromSecsSinceEpoch(
            static_cast<qint64>(report.value("started_at_unix").toDouble())).toString(Qt::ISODate);
        const QString result = QString::number(report.value("exit_code").toInt());
        const QString title = L("Report loaded", "Rapport chargé") + " · " + mode + " · " + date
            + " · " + L("exit", "code") + "=" + result;
        statusBar()->showMessage(title);
        const auto warnings = report.value("warnings").toArray();
        if (!warnings.isEmpty()) {
            QStringList messages;
            for (const auto &warning : warnings) messages << warning.toString();
            warning_->setText(messages.join("\n"));
        } else {
            updateMode();
        }
        tabs_->setCurrentWidget(files_);
        save_->setEnabled(true);
        filterRows();
    }

    void filterRows() {
        const QString needle = search_->text().trimmed();
        const QString selected = origin_->currentData().toString();
        for (auto *table : {files_, operations_, processes_, network_, sensitive_}) {
            const bool hasOrigin = table != files_;
            for (int row = 0; row < table->rowCount(); ++row) {
                bool matchesText = needle.isEmpty();
                for (int col = 0; col < table->columnCount(); ++col) {
                    const auto *cell = table->item(row, col);
                    if (cell && cell->text().contains(needle, Qt::CaseInsensitive)) matchesText = true;
                }
                const bool matchesOrigin = !hasOrigin || selected == "all" ||
                    (table->item(row, 0) && table->item(row, 0)->text() == selected);
                table->setRowHidden(row, !matchesText || !matchesOrigin);
            }
        }
    }

    void saveReport() {
        if (reportFile_.isEmpty() || !QFileInfo::exists(reportFile_)) return;
        const QString dest = QFileDialog::getSaveFileName(this, L("Save ShellTrace JSON", "Enregistrer le JSON ShellTrace"),
                                                           QDir::homePath() + "/shelltrace-report.json", "JSON (*.json)");
        if (dest.isEmpty()) return;
        if (QFileInfo(dest).absoluteFilePath() == QFileInfo(reportFile_).absoluteFilePath()) return;
        if (QFile::exists(dest)) {
            if (QMessageBox::question(this, "ShellTrace", L("Replace existing report?", "Remplacer le rapport existant?"),
                    QMessageBox::Yes | QMessageBox::Cancel, QMessageBox::Cancel) != QMessageBox::Yes) return;
            if (!QFile::remove(dest)) {
                QMessageBox::warning(this, "ShellTrace", L("Unable to replace the destination.", "Impossible de remplacer le fichier."));
                return;
            }
        }
        if (!QFile::copy(reportFile_, dest)) {
            QMessageBox::warning(this, "ShellTrace", L("Could not save the report.", "Impossible d'enregistrer le rapport."));
            return;
        }
        statusBar()->showMessage(L("Saved: ", "Enregistré : ") + dest);
    }

    void closeEvent(QCloseEvent *event) override {
        if (proc_.state() == QProcess::NotRunning) {
            event->accept();
            return;
        }
        const auto answer = QMessageBox::warning(this, "ShellTrace",
            L("An analysis is still running. Stop it and close? Some child processes could remain.",
              "Une analyse est en cours. L'arrêter et quitter ? Certains processus enfants peuvent subsister."),
            QMessageBox::Yes | QMessageBox::Cancel, QMessageBox::Cancel);
        if (answer == QMessageBox::Yes) {
            proc_.kill();
            proc_.waitForFinished(2000);
            event->accept();
        } else {
            event->ignore();
        }
    }

    QProcess proc_;
    QTemporaryDir temp_;
    const bool french_;
    const QPalette systemPalette_;
    bool logTruncated_ = false;
    QString reportFile_;

    QComboBox *mode_ = nullptr;
    QComboBox *theme_ = nullptr;
    QComboBox *origin_ = nullptr;
    QLineEdit *executable_ = nullptr;
    QLineEdit *arguments_ = nullptr;
    QLineEdit *scope_ = nullptr;
    QLineEdit *seed_ = nullptr;
    QLineEdit *engine_ = nullptr;
    QLineEdit *search_ = nullptr;
    QPushButton *run_ = nullptr;
    QPushButton *stop_ = nullptr;
    QPushButton *save_ = nullptr;
    QPushButton *open_ = nullptr;
    QLabel *warning_ = nullptr;
    QLabel *fileMetric_ = nullptr;
    QLabel *processMetric_ = nullptr;
    QLabel *networkMetric_ = nullptr;
    QLabel *sensitiveMetric_ = nullptr;
    QProgressBar *progress_ = nullptr;
    QTabWidget *tabs_ = nullptr;
    QTableWidget *files_ = nullptr;
    QTableWidget *operations_ = nullptr;
    QTableWidget *processes_ = nullptr;
    QTableWidget *network_ = nullptr;
    QTableWidget *sensitive_ = nullptr;
    QPlainTextEdit *log_ = nullptr;
    QPlainTextEdit *json_ = nullptr;
};

// Headless regression test: check the real Qt palette and QSS for text
// against its surrounding surface, rather than relying only on a compile test.
static double luminance(const QColor &color) {
    auto channel = [](double value) {
        return value <= 0.04045 ? value / 12.92 : std::pow((value + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * channel(color.redF()) +
           0.7152 * channel(color.greenF()) +
           0.0722 * channel(color.blueF());
}

static double contrastRatio(const QColor &first, const QColor &second) {
    const double a = luminance(first);
    const double b = luminance(second);
    return (qMax(a, b) + 0.05) / (qMin(a, b) + 0.05);
}

int main(int argc, char **argv) {
    QApplication app(argc, argv);
    app.setApplicationName("ShellTrace");
    app.setApplicationVersion("0.3.0");
    app.setOrganizationName("ShellTrace");
    QApplication::setStyle(QStyleFactory::create("Fusion"));

    const bool testContrast = app.arguments().contains("--self-test-contrast");
    if (testContrast) {
        QStandardPaths::setTestModeEnabled(true);
    }
    const QPalette originalPalette = QApplication::palette();
    ShellTraceWindow window;

    if (testContrast) {
        auto *selector = window.findChild<QComboBox*>("appearance-selector");
        if (!selector) {
            qCritical() << "Missing appearance selector";
            return 1;
        }
        for (const auto &mode : {"dark", "light"}) {
            const int index = selector->findData(mode);
            if (index < 0) return 2;
            selector->setCurrentIndex(index);
            QApplication::processEvents();
            const auto p = QApplication::palette();
            const double normal = contrastRatio(p.color(QPalette::Text), p.color(QPalette::Base));
            const double labels = contrastRatio(p.color(QPalette::WindowText), p.color(QPalette::Window));
            const double buttons = contrastRatio(p.color(QPalette::ButtonText), p.color(QPalette::Button));
            const double selected = contrastRatio(p.color(QPalette::HighlightedText), p.color(QPalette::Highlight));
            const QString css = QApplication::styleSheet();
            qInfo() << "Contrast test" << mode << normal << labels << buttons << selected;
            if (normal < 7.0 || labels < 7.0 || buttons < 7.0 || selected < 4.5) {
                qCritical() << "Insufficient text contrast in" << mode;
                return 3;
            }
            if (!css.contains("QComboBox QAbstractItemView") ||
                !css.contains("QTableWidget::item:selected") ||
                css.contains("@FG@") || css.contains("@SURFACE@")) {
                qCritical() << "Incomplete theme stylesheet for" << mode;
                return 4;
            }
        }
        const int system = selector->findData("system");
        selector->setCurrentIndex(system);
        QApplication::processEvents();
        if (QApplication::palette().color(QPalette::Text) != originalPalette.color(QPalette::Text) ||
            QApplication::palette().color(QPalette::Base) != originalPalette.color(QPalette::Base)) {
            qCritical() << "System theme did not restore the original Qt palette";
            return 5;
        }
        qInfo() << "Dark, Light and System theme contrast checks passed";
        return 0;
    }
    if (app.arguments().contains("--self-test")) {
        // Smoke test: construct all widgets without running any commands.
        return 0;
    }
    window.show();
    return app.exec();
}
