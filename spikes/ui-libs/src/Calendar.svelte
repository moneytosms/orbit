<script>
  // FullCalendar v7 vanilla core in a Svelte 5 attachment; no wrapper lib.
  import { Calendar } from 'fullcalendar';
  import timeGrid from 'fullcalendar/timegrid';
  import dayGrid from 'fullcalendar/daygrid';
  import interaction from 'fullcalendar/interaction';
  import classic from 'fullcalendar/themes/classic';
  import 'fullcalendar/skeleton.css';
  import 'fullcalendar/themes/classic/theme.css';
  import 'fullcalendar/themes/classic/palette.css';
  let log = $state([]);
  const say = (m) => (log = [m, ...log].slice(0, 30));
  const d = (h, m = 0, day = 0) => { const x = new Date(); x.setDate(x.getDate() + day); x.setHours(h, m, 0, 0); return x; };
  let events = $state([
    { id: '1', title: 'Deep work: thesis', start: d(9), end: d(11), color: 'var(--college)' },
    { id: '2', title: 'Gym', start: d(18), end: d(19), color: 'var(--learning)' },
    { id: '3', title: 'Pay rent (deadline)', start: d(0, 0, 1), allDay: true },
  ]);
  function mountCal(el) {
    const narrow = innerWidth < 600;
    const cal = new Calendar(el, {
      plugins: [classic, timeGrid, dayGrid, interaction],
      initialView: narrow ? 'timeGridDay' : 'timeGridWeek',
      headerToolbar: { start: 'prev,next today', center: 'title', end: 'timeGridWeek,timeGridDay,dayGridMonth' },
      height: narrow ? 560 : 640, nowIndicator: true, editable: true, selectable: true, firstDay: 1,
      longPressDelay: 300,
      events: $state.snapshot(events),
      eventDrop: (i) => say(`drop ${i.event.title} -> ${i.event.start.toLocaleString()}`),
      eventResize: (i) => say(`resize ${i.event.title} -> ends ${i.event.end.toLocaleTimeString()}`),
      select: (i) => { cal.addEvent({ id: crypto.randomUUID(), title: 'New block', start: i.start, end: i.end, allDay: i.allDay }); say(`select ${i.start.toLocaleString()} - ${i.end.toLocaleTimeString()}`); },
      eventClick: (i) => say(`click ${i.event.title}`),
    });
    cal.render();
    return () => cal.destroy();
  }
</script>
<p>Drag/resize events, drag on empty slots to create. Touch: long-press (300 ms) then drag.</p>
<div {@attach mountCal}></div>
<div class="log">{log.join('\n') || 'state log: no actions yet'}</div>
