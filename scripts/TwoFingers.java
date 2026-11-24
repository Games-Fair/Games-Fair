// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
// Synthetic test input for a local Android emulator; never packaged in the app.
import android.view.InputEvent;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.os.SystemClock;
import java.lang.reflect.Method;
public final class TwoFingers {
  static Object manager;
  static Method inject;
  static long down;
  static void send(int action, float[][] xy) throws Exception {
    MotionEvent.PointerProperties[] props = new MotionEvent.PointerProperties[xy.length];
    MotionEvent.PointerCoords[] coords = new MotionEvent.PointerCoords[xy.length];
    for (int i = 0; i < xy.length; i++) {
      props[i] = new MotionEvent.PointerProperties(); props[i].id = i; props[i].toolType = MotionEvent.TOOL_TYPE_FINGER;
      coords[i] = new MotionEvent.PointerCoords(); coords[i].x = xy[i][0]; coords[i].y = xy[i][1]; coords[i].pressure = 1; coords[i].size = 1;
    }
    MotionEvent event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action, xy.length, props, coords, 0, 0, 1, 1, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0);
    inject.invoke(manager, event, 2); event.recycle();
  }
  public static void main(String[] args) throws Exception {
    Class<?> cls = Class.forName("android.hardware.input.InputManager");
    manager = cls.getMethod("getInstance").invoke(null);
    inject = cls.getMethod("injectInputEvent", InputEvent.class, int.class);
    float[][] p = {{Float.parseFloat(args[0]), Float.parseFloat(args[1])}, {Float.parseFloat(args[2]), Float.parseFloat(args[3])}};
    float delta = Float.parseFloat(args[4]);
    down = SystemClock.uptimeMillis();
    send(MotionEvent.ACTION_DOWN, new float[][] {p[0]});
    send(MotionEvent.ACTION_POINTER_DOWN | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT), p);
    p[0][0] -= delta; p[1][0] += delta;
    send(MotionEvent.ACTION_MOVE, p);
    Thread.sleep(900);
    send(MotionEvent.ACTION_POINTER_UP | (1 << MotionEvent.ACTION_POINTER_INDEX_SHIFT), p);
    Thread.sleep(300);
    send(MotionEvent.ACTION_UP, new float[][] {p[0]});
  }
}
