// Texts of the phone's dashboard. Key: Czech. Row: en, sk, de, pl, es, fr.
import { translator, type Table } from './i18n.svelte';

const table: Table = {
  // Waiting screen
  'Telefon není spárovaný': ['Phone is not paired', 'Telefón nie je spárovaný', 'Telefon ist nicht gekoppelt', 'Telefon nie jest sparowany', 'El teléfono no está emparejado', "Le téléphone n'est pas associé"],
  'Na počítači otevři editor StreamPhone, záložku Připojení, a naskenuj QR kód.': [
    'On the computer, open StreamPhone, go to Connection and scan the QR code.',
    'Na počítači otvor StreamPhone, záložku Pripojenie, a naskenuj QR kód.',
    'Öffne StreamPhone am Computer, wechsle zu Verbindung und scanne den QR-Code.',
    'Na komputerze otwórz StreamPhone, zakładkę Połączenie, i zeskanuj kod QR.',
    'En el ordenador, abre StreamPhone, ve a Conexión y escanea el código QR.',
    "Sur l'ordinateur, ouvrez StreamPhone, onglet Connexion, et scannez le code QR.",
  ],
  'Máš v telefonu aplikaci StreamPhone? Spáruj telefon v ní.': [
    'Is the StreamPhone app on this phone? Pair the phone there.',
    'Máš v telefóne aplikáciu StreamPhone? Spáruj telefón v nej.',
    'Ist die StreamPhone-App auf diesem Telefon? Kopple das Telefon dort.',
    'Masz w telefonie aplikację StreamPhone? Sparuj telefon w niej.',
    '¿Tienes la aplicación StreamPhone en el teléfono? Empareja el teléfono allí.',
    "L'application StreamPhone est sur ce téléphone ? Associez le téléphone depuis celle-ci.",
  ],
  'Otevřít v aplikaci': ['Open in the app', 'Otvoriť v aplikácii', 'In der App öffnen', 'Otwórz w aplikacji', 'Abrir en la aplicación', "Ouvrir dans l'application"],
  'Pokračovat v prohlížeči': ['Continue in the browser', 'Pokračovať v prehliadači', 'Im Browser fortfahren', 'Kontynuuj w przeglądarce', 'Continuar en el navegador', 'Continuer dans le navigateur'],
  'Čekám na počítač…': ['Waiting for the computer…', 'Čakám na počítač…', 'Warte auf den Computer…', 'Czekam na komputer…', 'Esperando al ordenador…', "En attente de l'ordinateur…"],
  'Neznámý widget': ['Unknown widget', 'Neznámy widget', 'Unbekanntes Widget', 'Nieznany widżet', 'Widget desconocido', 'Widget inconnu'],
  'Telefon s Androidem': ['Android phone', 'Telefón s Androidom', 'Android-Telefon', 'Telefon z Androidem', 'Teléfono Android', 'Téléphone Android'],
  'Prohlížeč': ['Browser', 'Prehliadač', 'Browser', 'Przeglądarka', 'Navegador', 'Navigateur'],

  // AI sessions
  'AI relace': ['AI sessions', 'AI relácie', 'KI-Sitzungen', 'Sesje AI', 'Sesiones de IA', 'Sessions IA'],
  'Připraveno': ['Ready', 'Pripravené', 'Bereit', 'Gotowe', 'Listo', 'Prêt'],
  'Pracuje': ['Working', 'Pracuje', 'Arbeitet', 'Pracuje', 'Trabajando', 'En cours'],
  'Čeká na tebe': ['Waiting for you', 'Čaká na teba', 'Wartet auf dich', 'Czeka na ciebie', 'Te espera', 'Vous attend'],
  'Hotovo': ['Done', 'Hotovo', 'Fertig', 'Gotowe', 'Hecho', 'Terminé'],
  'teď': ['now', 'teraz', 'jetzt', 'teraz', 'ahora', 'maintenant'],
  'Relace': ['Session', 'Relácia', 'Sitzung', 'Sesja', 'Sesión', 'Session'],
  'Zamítnout': ['Deny', 'Zamietnuť', 'Ablehnen', 'Odrzuć', 'Denegar', 'Refuser'],
  'Povolit': ['Allow', 'Povoliť', 'Erlauben', 'Zezwól', 'Permitir', 'Autoriser'],
  'Žádná relace neběží': ['No session is running', 'Žiadna relácia nebeží', 'Keine Sitzung aktiv', 'Żadna sesja nie działa', 'No hay sesiones activas', 'Aucune session en cours'],

  // Discord
  'Server nemá zapnutý widget': ['The server has its widget turned off', 'Server nemá zapnutý widget', 'Das Server-Widget ist deaktiviert', 'Serwer ma wyłączony widżet', 'El servidor tiene el widget desactivado', 'Le widget du serveur est désactivé'],
  'Server s tímto ID neexistuje': ['No server has this ID', 'Server s týmto ID neexistuje', 'Kein Server mit dieser ID', 'Serwer o tym ID nie istnieje', 'No existe un servidor con este ID', 'Aucun serveur avec cet identifiant'],
  'Discord není dostupný': ['Discord is unreachable', 'Discord nie je dostupný', 'Discord ist nicht erreichbar', 'Discord jest niedostępny', 'Discord no está disponible', 'Discord est inaccessible'],
  'V editoru přidej ID serveru': ['Add a server ID in the editor', 'V editore pridaj ID servera', 'Füge im Editor eine Server-ID hinzu', 'Dodaj ID serwera w edytorze', 'Añade un ID de servidor en el editor', "Ajoutez un identifiant de serveur dans l'éditeur"],
  'Zjišťuji…': ['Checking…', 'Zisťujem…', 'Prüfe…', 'Sprawdzam…', 'Comprobando…', 'Vérification…'],
  'Nikdo z přátel není online': ['No friends are online', 'Nikto z priateľov nie je online', 'Keine Freunde online', 'Nikt ze znajomych nie jest online', 'Ningún amigo en línea', 'Aucun ami en ligne'],
  'Nikdo není online': ['Nobody is online', 'Nikto nie je online', 'Niemand ist online', 'Nikt nie jest online', 'Nadie en línea', 'Personne en ligne'],
  '{n} online': ['{n} online', '{n} online', '{n} online', '{n} online', '{n} en línea', '{n} en ligne'],

  // Media
  'Neznámá skladba': ['Unknown track', 'Neznáma skladba', 'Unbekannter Titel', 'Nieznany utwór', 'Pista desconocida', 'Titre inconnu'],
  'Nic nehraje': ['Nothing playing', 'Nič nehrá', 'Keine Wiedergabe', 'Nic nie gra', 'Nada en reproducción', 'Aucune lecture'],
  'Předchozí': ['Previous', 'Predchádzajúca', 'Zurück', 'Poprzedni', 'Anterior', 'Précédent'],
  'Přehrát / pauza': ['Play / pause', 'Prehrať / pauza', 'Wiedergabe / Pause', 'Odtwórz / pauza', 'Reproducir / pausa', 'Lecture / pause'],
  'Další': ['Next', 'Ďalšia', 'Weiter', 'Następny', 'Siguiente', 'Suivant'],
  'Ztlumit': ['Mute', 'Stlmiť', 'Stumm', 'Wycisz', 'Silenciar', 'Muet'],
  'Hlasitost': ['Volume', 'Hlasitosť', 'Lautstärke', 'Głośność', 'Volumen', 'Volume'],

  // Local models
  'Lokální modely': ['Local models', 'Lokálne modely', 'Lokale Modelle', 'Modele lokalne', 'Modelos locales', 'Modèles locaux'],
  'Neběží': ['Not running', 'Nebeží', 'Läuft nicht', 'Nie działa', 'Detenido', 'Arrêté'],
  '{n} v paměti': ['{n} loaded', '{n} v pamäti', '{n} geladen', '{n} w pamięci', '{n} en memoria', '{n} en mémoire'],
  'Nic v paměti': ['Nothing loaded', 'Nič v pamäti', 'Nichts geladen', 'Nic w pamięci', 'Nada en memoria', 'Rien en mémoire'],
  '{n} k dispozici': ['{n} available', '{n} k dispozícii', '{n} verfügbar', '{n} dostępnych', '{n} disponibles', '{n} disponibles'],
  '{n} na disku': ['{n} on disk', '{n} na disku', '{n} auf der Festplatte', '{n} na dysku', '{n} en disco', '{n} sur le disque'],
  'Žádný server neběží': ['No server is running', 'Žiadny server nebeží', 'Kein Server läuft', 'Żaden serwer nie działa', 'Ningún servidor activo', 'Aucun serveur actif'],

  // Processes, computer
  'Procesy': ['Processes', 'Procesy', 'Prozesse', 'Procesy', 'Procesos', 'Processus'],
  'Paměť': ['Memory', 'Pamäť', 'Speicher', 'Pamięć', 'Memoria', 'Mémoire'],
  'Ukončit': ['End', 'Ukončiť', 'Beenden', 'Zakończ', 'Finalizar', 'Arrêter'],
  'Načítám…': ['Loading…', 'Načítavam…', 'Lade…', 'Ładowanie…', 'Cargando…', 'Chargement…'],
  'Počítač': ['Computer', 'Počítač', 'Computer', 'Komputer', 'Ordenador', 'Ordinateur'],

  // Focus timer
  'Přestávka': ['Break', 'Prestávka', 'Pause', 'Przerwa', 'Descanso', 'Pause'],
  'Soustředění': ['Focus', 'Sústredenie', 'Fokus', 'Skupienie', 'Concentración', 'Concentration'],
  'Dokončená kola: {n}': ['Finished rounds: {n}', 'Dokončené kolá: {n}', 'Abgeschlossene Runden: {n}', 'Ukończone rundy: {n}', 'Rondas completadas: {n}', 'Cycles terminés : {n}'],
  'Od začátku': ['Start over', 'Od začiatku', 'Von vorn', 'Od początku', 'Reiniciar', 'Recommencer'],
  'Pozastavit': ['Pause', 'Pozastaviť', 'Pausieren', 'Wstrzymaj', 'Pausar', 'Mettre en pause'],
  'Spustit': ['Start', 'Spustiť', 'Starten', 'Start', 'Iniciar', 'Démarrer'],
  'Přeskočit': ['Skip', 'Preskočiť', 'Überspringen', 'Pomiń', 'Saltar', 'Passer'],

  // To-do list
  'Úkoly': ['Tasks', 'Úlohy', 'Aufgaben', 'Zadania', 'Tareas', 'Tâches'],
  'Vrátit': ['Undo', 'Vrátiť', 'Rückgängig', 'Cofnij', 'Deshacer', 'Annuler'],
  'Smazat': ['Delete', 'Zmazať', 'Löschen', 'Usuń', 'Eliminar', 'Supprimer'],
  'Nic na seznamu': ['Nothing on the list', 'Nič na zozname', 'Die Liste ist leer', 'Lista jest pusta', 'La lista está vacía', 'La liste est vide'],
  'Nový úkol': ['New task', 'Nová úloha', 'Neue Aufgabe', 'Nowe zadanie', 'Nueva tarea', 'Nouvelle tâche'],
  'Přidat': ['Add', 'Pridať', 'Hinzufügen', 'Dodaj', 'Añadir', 'Ajouter'],
};

export const t = translator(table);
export const keys = Object.keys(table);
