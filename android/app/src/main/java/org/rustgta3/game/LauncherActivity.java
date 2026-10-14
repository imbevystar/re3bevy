package org.rustgta3.game;

import android.app.Activity;
import android.content.Intent;
import android.content.res.AssetManager;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.os.StatFs;
import android.provider.DocumentsContract;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.TextView;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Locale;
import java.util.Set;

/** Import user-provided data before native startup. Explicit private test
 * APKs may still supply bundled data. Saves live outside this data directory. */
public final class LauncherActivity extends Activity {
    private static final Object INSTALL_LOCK = new Object();
    private static final int SELECT_GAME_DATA = 1;
    private static final Set<String> DATA_DIRECTORIES = new HashSet<>(Arrays.asList(
        "anim", "audio", "data", "models", "movies", "mp3", "skins", "text", "txd"));
    private static final String[] REQUIRED_FILES = {
        "models/coll/peds.col", "models/gta3.img", "models/gta3.dir",
        "data/default.dat", "data/gta3.dat", "audio/sfx.raw", "audio/sfx.sdt"
    };
    private TextView status;
    private ProgressBar progress;
    private Button chooseData;
    private volatile boolean destroyed;
    private long copied;
    private long lastUpdate;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setGravity(Gravity.CENTER);
        panel.setPadding(48, 24, 48, 24);
        status = new TextView(this);
        status.setText("Preparing GTA III data…");
        status.setTextSize(20);
        progress = new ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal);
        progress.setMax(1000);
        panel.addView(status);
        panel.addView(progress, new LinearLayout.LayoutParams(-1, 32));
        chooseData = new Button(this);
        chooseData.setText("Choose GTA III data folder");
        chooseData.setVisibility(View.GONE);
        chooseData.setOnClickListener(view -> {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            startActivityForResult(intent, SELECT_GAME_DATA);
        });
        panel.addView(chooseData);
        setContentView(panel);
        prepareData(null);
    }

    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != SELECT_GAME_DATA) return;
        if (result == RESULT_OK && data != null && data.getData() != null) {
            chooseData.setVisibility(View.GONE);
            progress.setVisibility(View.VISIBLE);
            progress.setIndeterminate(true);
            status.setText("Importing your game data…");
            prepareData(data.getData());
        }
    }

    private void showImport(String message) {
        runOnUiThread(() -> { if (!destroyed) {
            status.setText(message);
            progress.setVisibility(View.GONE);
            chooseData.setVisibility(View.VISIBLE);
        }});
    }

    private void prepareData(Uri source) {
        new Thread(() -> {
            try {
                synchronized (INSTALL_LOCK) {
                    copied = 0;
                    recoverInstallation();
                    if (source != null) {
                        importDocuments(source);
                    } else if (Arrays.asList(getAssets().list("")).contains("gta3-revision")) {
                        installData();
                    } else if (!new File(getFilesDir(), "gta3").isDirectory()) {
                        showImport("Supply data from your own PC copy of GTA III.\n"
                            + "Choose the folder containing anim, audio, data, models and text.");
                        return;
                    }
                    validateData(new File(getFilesDir(), "gta3"));
                }
                runOnUiThread(() -> {
                    if (!destroyed) { startActivity(new Intent(this, MainActivity.class)); finish(); }
                });
            } catch (Exception error) {
                showImport("Could not prepare game data:\n" + error.getMessage()
                    + "\nCheck the selected PC installation and free storage, then try again.");
            }
        }, "GTA data installation").start();
    }

    @Override public void onDestroy() { destroyed = true; super.onDestroy(); }

    private void installData() throws IOException {
        AssetManager assets = getAssets();
        String revision;
        try (InputStream stream = assets.open("gta3-revision")) {
            revision = readText(stream).trim();
        }
        String[] fields = revision.split(" ");
        if (fields.length != 2) throw new IOException("Invalid APK game data revision");
        long total = Long.parseLong(fields[1]);
        File root = new File(getFilesDir(), "gta3");
        File marker = new File(root, ".apk-revision");
        if (marker.isFile()) {
            try (InputStream stream = new FileInputStream(marker)) {
                if (readText(stream).trim().equals(revision)) return;
            }
        }
        File pending = new File(getFilesDir(), "gta3-installing");
        deleteTree(pending);
        if (new StatFs(getFilesDir().getPath()).getAvailableBytes() < total + 64L * 1024 * 1024)
            throw new IOException("Need " + (total / (1024 * 1024) + 64) + " MB of free internal storage");
        if (!pending.mkdirs()) throw new IOException("Cannot create game data directory");
        try {
            copyAssets(assets, "gta3", pending, total);
            if (copied != total) throw new IOException("APK game data size mismatch");
            try (FileOutputStream stream = new FileOutputStream(new File(pending, ".apk-revision"))) {
                stream.write(revision.getBytes(StandardCharsets.UTF_8));
                stream.getFD().sync();
            }
            validateData(pending);
            commitData(pending);
        } catch (IOException error) {
            deleteTree(pending);
            throw error;
        }
    }

    private void importDocuments(Uri tree) throws IOException {
        File pending = new File(getFilesDir(), "gta3-installing");
        deleteTree(pending);
        if (!pending.mkdirs()) throw new IOException("Cannot create game data directory");
        try {
            copyDocuments(tree, DocumentsContract.getTreeDocumentId(tree), pending, true, 0);
            validateData(pending);
            commitData(pending);
        } catch (IOException | RuntimeException error) {
            deleteTree(pending);
            throw error;
        }
    }

    private void copyDocuments(Uri tree, String parentId, File destination, boolean root, int depth)
            throws IOException {
        if (depth > 32) throw new IOException("Game data folder nesting is too deep");
        Uri children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parentId);
        String[] columns = { DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME, DocumentsContract.Document.COLUMN_MIME_TYPE };
        try (Cursor cursor = getContentResolver().query(children, columns, null, null, null)) {
            if (cursor == null) throw new IOException("Cannot read selected game data folder");
            while (cursor.moveToNext()) {
                if (destroyed) throw new IOException("Game data import was cancelled");
                String id = cursor.getString(0);
                String name = cursor.getString(1);
                boolean directory = DocumentsContract.Document.MIME_TYPE_DIR.equals(cursor.getString(2));
                if (name == null || name.isEmpty() || name.equals(".") || name.equals("..")
                        || name.indexOf('/') >= 0 || name.indexOf('\\') >= 0 || name.indexOf('\0') >= 0)
                    throw new IOException("Invalid name in selected game data folder");
                // Import only data directories, never the PC executable or DLLs.
                if (root && (!directory || !DATA_DIRECTORIES.contains(name.toLowerCase(Locale.ROOT)))) continue;
                File target = new File(destination, name);
                if (directory) {
                    if (!target.mkdir()) throw new IOException("Cannot create game data folder " + name);
                    copyDocuments(tree, id, target, false, depth + 1);
                } else {
                    if (new StatFs(getFilesDir().getPath()).getAvailableBytes() < 64L * 1024 * 1024)
                        throw new IOException("Not enough free internal storage to import game data");
                    Uri document = DocumentsContract.buildDocumentUriUsingTree(tree, id);
                    try (InputStream input = getContentResolver().openInputStream(document)) {
                        if (input == null) throw new IOException("Cannot open game data file " + name);
                        copyFile(input, target, -1);
                    }
                }
            }
        }
    }

    private static void validateData(File root) throws IOException {
        for (String required : REQUIRED_FILES) {
            File file = root;
            for (String part : required.split("/")) {
                File exact = new File(file, part);
                if (exact.exists()) { file = exact; continue; }
                File[] entries = file.listFiles();
                file = exact;
                if (entries != null) for (File entry : entries)
                    if (entry.getName().equalsIgnoreCase(part)) { file = entry; break; }
            }
            if (!file.isFile()) throw new IOException("Missing original GTA III data: " + required);
        }
    }

    private void recoverInstallation() throws IOException {
        File root = new File(getFilesDir(), "gta3");
        File previous = new File(getFilesDir(), "gta3-previous");
        if (!previous.exists()) return;
        if (root.exists()) deleteTree(previous);
        else Files.move(previous.toPath(), root.toPath(), StandardCopyOption.ATOMIC_MOVE);
    }

    private void commitData(File pending) throws IOException {
        File root = new File(getFilesDir(), "gta3");
        File previous = new File(getFilesDir(), "gta3-previous");
        if (root.exists()) Files.move(root.toPath(), previous.toPath(), StandardCopyOption.ATOMIC_MOVE);
        try {
            Files.move(pending.toPath(), root.toPath(), StandardCopyOption.ATOMIC_MOVE);
        } catch (IOException error) {
            if (previous.exists()) {
                try { Files.move(previous.toPath(), root.toPath(), StandardCopyOption.ATOMIC_MOVE); }
                catch (IOException restoreError) { error.addSuppressed(restoreError); }
            }
            throw error;
        }
        deleteTree(previous);
    }

    private static String readText(InputStream input) throws IOException {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        byte[] buffer = new byte[1024];
        int count;
        while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
        return new String(output.toByteArray(), StandardCharsets.UTF_8);
    }

    private void copyAssets(AssetManager assets, String source, File destination, long total) throws IOException {
        String[] children = assets.list(source);
        if (children == null) throw new IOException("Cannot list APK asset " + source);
        for (String child : children) {
            String asset = source + "/" + child;
            File target = new File(destination, child);
            String[] contents = assets.list(asset);
            if (contents != null && contents.length != 0) {
                if (!target.mkdirs()) throw new IOException("Cannot create " + target);
                copyAssets(assets, asset, target, total);
            } else {
                try (InputStream input = assets.open(asset)) {
                    copyFile(input, target, total);
                }
            }
        }
    }

    private void copyFile(InputStream input, File target, long total) throws IOException {
        byte[] buffer = new byte[128 * 1024];
        try (FileOutputStream output = new FileOutputStream(target)) {
            int count;
            while ((count = input.read(buffer)) != -1) {
                if (destroyed) throw new IOException("Game data import was cancelled");
                output.write(buffer, 0, count);
                copied += count;
                if (System.currentTimeMillis() - lastUpdate > 150) {
                    lastUpdate = System.currentTimeMillis();
                    final long imported = copied;
                    runOnUiThread(() -> { if (!destroyed) {
                        progress.setIndeterminate(total < 0);
                        if (total >= 0) progress.setProgress((int)(imported * 1000 / Math.max(1, total)));
                        status.setText("Preparing GTA III data: " + imported / (1024 * 1024) + " MB");
                    }});
                }
            }
            output.getFD().sync();
        }
    }

    private static void deleteTree(File root) throws IOException {
        if (!root.exists()) return;
        File[] children = Files.isSymbolicLink(root.toPath()) ? null : root.listFiles();
        if (children != null) for (File child : children) deleteTree(child);
        if (!root.delete()) throw new IOException("Cannot remove incomplete game data " + root);
    }
}
