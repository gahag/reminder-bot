create table reminders (
	id         integer not null primary key,
	due        bigint  not null, -- Unix time
	recurrying integer, -- Recurrence custom format.
	chat       bigint  not null,
	message    text    not null
);

create index reminders_due on reminders (due);
